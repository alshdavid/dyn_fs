use std::io;
use std::io::Cursor;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use ::include_dir::Dir;
use ::include_dir::DirEntry as EmbeddedDirEntry;
use ::include_dir::File as EmbeddedFile;

use super::IncludeDirDirEntry;
use super::IncludeDirFile;
use super::IncludeDirFileType;
use super::IncludeDirMetadata;
use super::IncludeDirReadDir;
use crate::DirBuilder;
use crate::File;
use crate::FileSystem;
use crate::FileTimes;
use crate::Metadata;
use crate::OpenOptions;
use crate::Permissions;
use crate::ReadDir;

/// A read-only [`FileSystem`] backed by an embedded [`include_dir`] directory.
///
/// Only the read-only portions of the interface are implemented: reads,
/// metadata, directory listing and existence checks. Every mutating operation
/// (create/remove/rename/write/symlink/...) returns
/// [`io::ErrorKind::Unsupported`].
///
/// Paths are resolved relative to the embedded root directory. Because
/// `include_dir` records entry paths relative to the directory passed to
/// [`include_dir!`](::include_dir::include_dir), the root's own path is stripped
/// so that wrapping a sub-directory exposes its contents at the file system
/// root.
#[derive(Debug, Clone)]
pub struct IncludeDirFileSystem {
  root: Dir<'static>,
  prefix: PathBuf,
}

impl IncludeDirFileSystem {
  /// Creates a file system for the given embedded directory, returned as a
  /// shared [`Arc`] trait object.
  ///
  /// Paths are resolved relative to `root`. Because `include_dir` records the
  /// paths of entries relative to the directory passed to
  /// [`include_dir!`](::include_dir::include_dir), passing a sub-directory
  /// (for example `ASSETS.get_dir("assets").unwrap().clone()`) exposes that
  /// sub-directory's contents at the file system root.
  #[allow(clippy::new_ret_no_self)]
  #[must_use]
  pub fn new(root: Dir<'static>) -> Arc<dyn FileSystem> {
    let prefix = root.path().to_path_buf();
    Arc::new(Self { root, prefix })
  }

  /// Returns the embedded root directory.
  #[must_use]
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
  ) -> IncludeDirMetadata {
    match entry {
      EmbeddedDirEntry::Dir(_) => IncludeDirMetadata {
        file_type: IncludeDirFileType::directory(),
        len: 0,
      },
      EmbeddedDirEntry::File(file) => IncludeDirMetadata {
        file_type: IncludeDirFileType::file(),
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

impl FileSystem for IncludeDirFileSystem {
  fn open_with(
    &self,
    path: &Path,
    options: &OpenOptions,
  ) -> io::Result<Box<dyn File + Send + Sync>> {
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
      Some(file) => Ok(Box::new(IncludeDirFile {
        cursor: Cursor::new(file.contents()),
      })),
      None => Err(not_found()),
    }
  }

  fn canonicalize(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf> {
    let key = self.key(path);
    self.resolve_entry(&key)?;
    Ok(self.expose(&key))
  }

  fn read_link(
    &self,
    _path: &Path,
  ) -> io::Result<PathBuf> {
    Err(unsupported("read_link"))
  }

  fn copy(
    &self,
    _from: &Path,
    _to: &Path,
  ) -> io::Result<u64> {
    Err(unsupported("copy"))
  }

  fn create_dir(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    Err(unsupported("create_dir"))
  }

  fn create_dir_all(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    Err(unsupported("create_dir_all"))
  }

  fn create_dir_with(
    &self,
    _path: &Path,
    _builder: &DirBuilder,
  ) -> io::Result<()> {
    Err(unsupported("create_dir_with"))
  }

  fn hard_link(
    &self,
    _original: &Path,
    _link: &Path,
  ) -> io::Result<()> {
    Err(unsupported("hard_link"))
  }

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let key = self.key(path);

    match self.resolve_entry(&key)? {
      Some(entry) => Ok(Box::new(self.entry_metadata(entry))),
      None => Ok(Box::new(IncludeDirMetadata {
        file_type: IncludeDirFileType::directory(),
        len: 0,
      })),
    }
  }

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>> {
    let key = self.key(path);

    match self.resolve_file(&key)? {
      Some(file) => Ok(file.contents().to_vec()),
      None => Err(not_found()),
    }
  }

  fn read_dir(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn ReadDir + Send + Sync>> {
    let key = self.key(path);
    let entries = self.resolve_dir(&key)?;

    let entries: Vec<IncludeDirDirEntry> = entries
      .iter()
      .map(|entry| IncludeDirDirEntry {
        path: self.expose(entry.path()),
        metadata: self.entry_metadata(entry),
      })
      .collect();

    Ok(Box::new(IncludeDirReadDir::from(entries)))
  }

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String> {
    let bytes = self.read(path)?;
    String::from_utf8(bytes)
      .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not valid UTF-8"))
  }

  fn remove_dir(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    Err(unsupported("remove_dir"))
  }

  fn remove_dir_all(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    Err(unsupported("remove_dir_all"))
  }

  fn remove_file(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    Err(unsupported("remove_file"))
  }

  fn rename(
    &self,
    _from: &Path,
    _to: &Path,
  ) -> io::Result<()> {
    Err(unsupported("rename"))
  }

  fn set_permissions(
    &self,
    _path: &Path,
    _perm: &Permissions,
  ) -> io::Result<()> {
    Err(unsupported("set_permissions"))
  }

  fn set_times(
    &self,
    _path: &Path,
    _times: &FileTimes,
  ) -> io::Result<()> {
    Err(unsupported("set_times"))
  }

  fn symlink(
    &self,
    _original: &Path,
    _link: &Path,
  ) -> io::Result<()> {
    Err(unsupported("symlink"))
  }

  fn symlink_metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    self.metadata(path)
  }

  fn try_exists(
    &self,
    path: &Path,
  ) -> io::Result<bool> {
    let relative = self.normalize(path);

    if relative.as_os_str().is_empty() {
      return Ok(true);
    }

    Ok(self.root.contains(self.prefix.join(&relative)))
  }

  fn write(
    &self,
    _path: &Path,
    _contents: &[u8],
  ) -> io::Result<()> {
    Err(unsupported("write"))
  }
}
