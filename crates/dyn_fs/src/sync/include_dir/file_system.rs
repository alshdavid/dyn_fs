use std::io;
use std::io::Cursor;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use ::include_dir::Dir;
use ::include_dir::DirEntry as EmbeddedDirEntry;
use ::include_dir::File as EmbeddedFile;

use super::IncludeDirDirEntry;
use super::IncludeDirFile;
use super::IncludeDirFileType;
use super::IncludeDirMetadata;
use super::IncludeDirReadDir;
use crate::file_system::DirBuilder;
use crate::file_system::File;
use crate::file_system::FileSystem;
use crate::file_system::FileTimes;
use crate::file_system::Metadata;
use crate::file_system::OpenOptions;
use crate::file_system::Permissions;
use crate::file_system::ReadDir;

/// A read-only [`FileSystem`] backed by an embedded [`include_dir`] directory.
///
/// Only the read-only portions of the interface are implemented: reads,
/// metadata, directory listing and existence checks. Every mutating operation
/// (create/remove/rename/write/symlink/...) returns
/// [`io::ErrorKind::Unsupported`].
///
/// The paths exposed by this file system are the paths recorded by
/// [`include_dir!`](::include_dir::include_dir), which are relative to the
/// directory passed to the macro. A mount point may be supplied with
/// [`IncludeDirFileSystem::with_mount`] to place the embedded tree under a
/// prefix.
#[derive(Debug, Clone)]
pub struct IncludeDirFileSystem {
  root: Dir<'static>,
  mount: PathBuf,
}

impl IncludeDirFileSystem {
  /// Creates a file system for the given embedded directory.
  ///
  /// Paths are resolved relative to the directory passed to
  /// [`include_dir!`](::include_dir::include_dir).
  pub fn new(root: Dir<'static>) -> Self {
    Self {
      root,
      mount: PathBuf::new(),
    }
  }

  /// Creates a file system that exposes the embedded directory under `mount`.
  pub fn with_mount(
    root: Dir<'static>,
    mount: impl Into<PathBuf>,
  ) -> Self {
    Self {
      root,
      mount: mount.into(),
    }
  }

  /// Returns the embedded root directory.
  pub fn root(&self) -> &Dir<'static> {
    &self.root
  }

  /// Returns the mount point the embedded directory is exposed under.
  pub fn mount(&self) -> &Path {
    &self.mount
  }

  /// Maps an exposed path to a path relative to the embedded root.
  fn relative(
    &self,
    path: &Path,
  ) -> PathBuf {
    let stripped = if self.mount.as_os_str().is_empty() {
      path
    } else {
      path.strip_prefix(&self.mount).unwrap_or(path)
    };

    let mut relative = PathBuf::new();

    for component in stripped.components() {
      match component {
        Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        component => relative.push(component),
      }
    }

    relative
  }

  /// Maps a path relative to the embedded root back to an exposed path.
  fn absolute(
    &self,
    path: &Path,
  ) -> PathBuf {
    if self.mount.as_os_str().is_empty() {
      path.to_path_buf()
    } else {
      self.mount.join(path)
    }
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

  /// Resolves a directory to its list of embedded entries.
  fn resolve_dir(
    &self,
    relative: &Path,
  ) -> io::Result<&'static [EmbeddedDirEntry<'static>]> {
    if relative.as_os_str().is_empty() {
      return Ok(self.root.entries());
    }

    if let Some(dir) = self.root.get_dir(relative) {
      return Ok(dir.entries());
    }

    if self.root.get_file(relative).is_some() {
      return Err(not_a_directory());
    }

    Err(not_found())
  }

  /// Resolves a file to its embedded representation.
  fn resolve_file(
    &self,
    relative: &Path,
  ) -> io::Result<Option<&'static EmbeddedFile<'static>>> {
    if relative.as_os_str().is_empty() {
      return Err(is_a_directory());
    }

    if let Some(file) = self.root.get_file(relative) {
      return Ok(Some(file));
    }

    if self.root.get_dir(relative).is_some() {
      return Err(is_a_directory());
    }

    Err(not_found())
  }

  /// Resolves any path to its embedded entry.
  fn resolve_entry(
    &self,
    relative: &Path,
  ) -> io::Result<Option<&'static EmbeddedDirEntry<'static>>> {
    if relative.as_os_str().is_empty() {
      return Ok(None);
    }

    match self.root.get_entry(relative) {
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

    let relative = self.relative(path);

    match self.resolve_file(&relative)? {
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
    let relative = self.relative(path);
    self.resolve_entry(&relative)?;
    Ok(self.absolute(&relative))
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
    let relative = self.relative(path);

    match self.resolve_entry(&relative)? {
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
    let relative = self.relative(path);

    match self.resolve_file(&relative)? {
      Some(file) => Ok(file.contents().to_vec()),
      None => Err(not_found()),
    }
  }

  fn read_dir(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn ReadDir + Send + Sync>> {
    let relative = self.relative(path);
    let entries = self.resolve_dir(&relative)?;

    let entries: Vec<IncludeDirDirEntry> = entries
      .iter()
      .map(|entry| IncludeDirDirEntry {
        path: self.absolute(entry.path()),
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
    let relative = self.relative(path);

    if relative.as_os_str().is_empty() {
      return Ok(true);
    }

    Ok(self.root.contains(relative))
  }

  fn write(
    &self,
    _path: &Path,
    _contents: &[u8],
  ) -> io::Result<()> {
    Err(unsupported("write"))
  }
}
