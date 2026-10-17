use std::io;
use std::io::Cursor;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use ::include_dir::Dir;
use ::include_dir::DirEntry as EmbeddedDirEntry;
use ::include_dir::File as EmbeddedFile;

use super::AsyncIncludeDirDirEntry;
use super::AsyncIncludeDirFile;
use super::AsyncIncludeDirFileType;
use super::AsyncIncludeDirMetadata;
use super::AsyncIncludeDirReadDir;
use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncFile;
use crate::asynch::file_system::AsyncFileSystem;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::AsyncReadDir;
use crate::asynch::file_system::BoxFuture;
use crate::asynch::file_system::DirBuilder;
use crate::asynch::file_system::FileTimes;
use crate::asynch::file_system::OpenOptions;
use crate::asynch::file_system::Permissions;

/// A read-only [`AsyncFileSystem`] backed by an embedded [`include_dir`]
/// directory.
///
/// Only the read-only portions of the interface are implemented: reads,
/// metadata, directory listing and existence checks. Every mutating operation
/// (create/remove/rename/write/symlink/...) returns
/// [`io::ErrorKind::Unsupported`].
///
/// Because the embedded data lives in memory, every returned future is ready on
/// its first poll and never spawns work.
///
/// Paths are resolved relative to the embedded root directory. Because
/// `include_dir` records entry paths relative to the directory passed to
/// [`include_dir!`](::include_dir::include_dir), the root's own path is stripped
/// so that wrapping a sub-directory exposes its contents at the file system
/// root.
#[derive(Debug, Clone)]
pub struct AsyncIncludeDirFileSystem {
  root: Dir<'static>,
  prefix: PathBuf,
}

impl AsyncIncludeDirFileSystem {
  /// Creates a file system for the given embedded directory, returned as a
  /// shared [`Arc`] trait object.
  ///
  /// Paths are resolved relative to `root`. Because `include_dir` records the
  /// paths of entries relative to the directory passed to
  /// [`include_dir!`](::include_dir::include_dir), passing a sub-directory
  /// (for example `ASSETS.get_dir("assets").unwrap().clone()`) exposes that
  /// sub-directory's contents at the file system root.
  #[allow(clippy::new_ret_no_self)]
  pub fn new(root: Dir<'static>) -> Arc<dyn AsyncFileSystem> {
    let prefix = root.path().to_path_buf();
    Arc::new(Self { root, prefix })
  }

  /// Returns the embedded root directory.
  pub fn root(&self) -> &Dir<'static> {
    &self.root
  }

  /// Removes redundant components (`/`, `.`) from an exposed path.
  fn normalize(
    &self,
    path: &Path,
  ) -> PathBuf {
    let mut relative = PathBuf::new();

    for component in path.components() {
      match component {
        Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        component => relative.push(component),
      }
    }

    relative
  }

  /// Maps an exposed path to the lookup key used against the embedded tree (a
  /// path relative to the original `include_dir!` root).
  fn key(
    &self,
    path: &Path,
  ) -> PathBuf {
    self.prefix.join(self.normalize(path))
  }

  /// Maps a lookup key back to an exposed path.
  fn expose(
    &self,
    key: &Path,
  ) -> PathBuf {
    key.strip_prefix(&self.prefix).unwrap_or(key).to_path_buf()
  }

  /// Produces metadata for an embedded entry.
  fn entry_metadata(
    &self,
    entry: &EmbeddedDirEntry<'static>,
  ) -> AsyncIncludeDirMetadata {
    match entry {
      EmbeddedDirEntry::Dir(_) => AsyncIncludeDirMetadata {
        file_type: AsyncIncludeDirFileType::directory(),
        len: 0,
      },
      EmbeddedDirEntry::File(file) => AsyncIncludeDirMetadata {
        file_type: AsyncIncludeDirFileType::file(),
        len: file.contents().len() as u64,
      },
    }
  }

  /// Resolves a lookup key to its directory entries.
  fn resolve_dir(
    &self,
    key: &Path,
  ) -> io::Result<&'static [EmbeddedDirEntry<'static>]> {
    if key == self.prefix {
      return Ok(self.root.entries());
    }

    if let Some(dir) = self.root.get_dir(key) {
      return Ok(dir.entries());
    }

    if self.root.get_file(key).is_some() {
      return Err(not_a_directory());
    }

    Err(not_found())
  }

  /// Resolves a lookup key to its embedded file.
  fn resolve_file(
    &self,
    key: &Path,
  ) -> io::Result<Option<&'static EmbeddedFile<'static>>> {
    if key == self.prefix {
      return Err(is_a_directory());
    }

    if let Some(file) = self.root.get_file(key) {
      return Ok(Some(file));
    }

    if self.root.get_dir(key).is_some() {
      return Err(is_a_directory());
    }

    Err(not_found())
  }

  /// Resolves a lookup key to its embedded entry.
  fn resolve_entry(
    &self,
    key: &Path,
  ) -> io::Result<Option<&'static EmbeddedDirEntry<'static>>> {
    if key == self.prefix {
      return Ok(None);
    }

    match self.root.get_entry(key) {
      Some(entry) => Ok(Some(entry)),
      None => Err(not_found()),
    }
  }
}

fn not_found() -> io::Error {
  io::Error::new(io::ErrorKind::NotFound, "path not found")
}

fn not_a_directory() -> io::Error {
  io::Error::new(io::ErrorKind::NotADirectory, "not a directory")
}

fn is_a_directory() -> io::Error {
  io::Error::new(io::ErrorKind::IsADirectory, "is a directory")
}

fn unsupported(operation: &str) -> io::Error {
  io::Error::new(
    io::ErrorKind::Unsupported,
    format!("include_dir file system does not support {operation}"),
  )
}

impl AsyncFileSystem for AsyncIncludeDirFileSystem {
  fn open_with<'a>(
    &'a self,
    path: &'a Path,
    options: OpenOptions,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncFile + Send + Sync>>> {
    Box::pin(async move {
      if options.get_write()
        || options.get_append()
        || options.get_truncate()
        || options.get_create()
        || options.get_create_new()
      {
        return Err(unsupported("writing"));
      }

      let key = self.key(path);

      match self.resolve_file(&key)? {
        Some(file) => Ok(Box::new(AsyncIncludeDirFile {
          cursor: Cursor::new(file.contents()),
        }) as Box<dyn AsyncFile + Send + Sync>),
        None => Err(not_found()),
      }
    })
  }

  fn canonicalize<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>> {
    Box::pin(async move {
      let key = self.key(path);
      self.resolve_entry(&key)?;
      Ok(self.expose(&key))
    })
  }

  fn read_link<'a>(
    &'a self,
    _path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>> {
    Box::pin(async move { Err(unsupported("read_link")) })
  }

  fn copy<'a>(
    &'a self,
    _from: &'a Path,
    _to: &'a Path,
  ) -> BoxFuture<'a, io::Result<u64>> {
    Box::pin(async move { Err(unsupported("copy")) })
  }

  fn create_dir<'a>(
    &'a self,
    _path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("create_dir")) })
  }

  fn create_dir_all<'a>(
    &'a self,
    _path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("create_dir_all")) })
  }

  fn create_dir_with<'a>(
    &'a self,
    _path: &'a Path,
    _builder: DirBuilder,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("create_dir_with")) })
  }

  fn hard_link<'a>(
    &'a self,
    _original: &'a Path,
    _link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("hard_link")) })
  }

  fn metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let key = self.key(path);

      match self.resolve_entry(&key)? {
        Some(entry) => {
          Ok(Box::new(self.entry_metadata(entry)) as Box<dyn AsyncMetadata + Send + Sync>)
        }
        None => Ok(Box::new(AsyncIncludeDirMetadata {
          file_type: AsyncIncludeDirFileType::directory(),
          len: 0,
        }) as Box<dyn AsyncMetadata + Send + Sync>),
      }
    })
  }

  fn read<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Vec<u8>>> {
    Box::pin(async move {
      let key = self.key(path);

      match self.resolve_file(&key)? {
        Some(file) => Ok(file.contents().to_vec()),
        None => Err(not_found()),
      }
    })
  }

  fn read_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncReadDir>>> {
    Box::pin(async move {
      let key = self.key(path);
      let entries = self.resolve_dir(&key)?;

      let entries: Vec<Box<dyn AsyncDirEntry + Send + Sync>> = entries
        .iter()
        .map(|entry| {
          Box::new(AsyncIncludeDirDirEntry {
            path: self.expose(entry.path()),
            metadata: self.entry_metadata(entry),
          }) as Box<dyn AsyncDirEntry + Send + Sync>
        })
        .collect();

      Ok(Box::new(AsyncIncludeDirReadDir::from(entries)) as Box<dyn AsyncReadDir>)
    })
  }

  fn read_to_string<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<String>> {
    Box::pin(async move {
      let key = self.key(path);

      let bytes = match self.resolve_file(&key)? {
        Some(file) => file.contents().to_vec(),
        None => return Err(not_found()),
      };

      String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not valid UTF-8"))
    })
  }

  fn remove_dir<'a>(
    &'a self,
    _path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("remove_dir")) })
  }

  fn remove_dir_all<'a>(
    &'a self,
    _path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("remove_dir_all")) })
  }

  fn remove_file<'a>(
    &'a self,
    _path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("remove_file")) })
  }

  fn rename<'a>(
    &'a self,
    _from: &'a Path,
    _to: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("rename")) })
  }

  fn set_permissions<'a>(
    &'a self,
    _path: &'a Path,
    _perm: Permissions,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("set_permissions")) })
  }

  fn set_times<'a>(
    &'a self,
    _path: &'a Path,
    _times: FileTimes,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("set_times")) })
  }

  fn symlink<'a>(
    &'a self,
    _original: &'a Path,
    _link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("symlink")) })
  }

  fn symlink_metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    self.metadata(path)
  }

  fn try_exists<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<bool>> {
    Box::pin(async move {
      let relative = self.normalize(path);

      if relative.as_os_str().is_empty() {
        return Ok(true);
      }

      Ok(self.root.contains(self.prefix.join(&relative)))
    })
  }

  fn write<'a>(
    &'a self,
    _path: &'a Path,
    _contents: &'a [u8],
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { Err(unsupported("write")) })
  }
}
