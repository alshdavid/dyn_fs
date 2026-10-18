//! Abstraction of the file system
//!
//! This module contains traits that map the functionality of
//! the Rust standard library `FileSystem` to trait methods

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

pub type FileSystemRef = Arc<dyn FileSystem>;

pub trait FileSystem: Send + Sync {
  /// Opens a file in read-only mode.
  fn open(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn File + Send + Sync>> {
    self.open_with(path, OpenOptions::new().read(true))
  }

  /// Opens a file with the given options.
  fn open_with(
    &self,
    path: &Path,
    options: &OpenOptions,
  ) -> io::Result<Box<dyn File + Send + Sync>>;

  fn canonicalize(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf>;

  fn read_link(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf>;

  fn copy(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<u64>;

  fn create_dir(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn create_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  /// Creates a directory using the given builder.
  fn create_dir_with(
    &self,
    path: &Path,
    builder: &DirBuilder,
  ) -> io::Result<()>;

  fn hard_link(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()>;

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>>;

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>>;

  fn read_dir(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn ReadDir + Send + Sync>>;

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String>;

  fn remove_dir(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn remove_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn remove_file(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn rename(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<()>;

  fn set_permissions(
    &self,
    path: &Path,
    perm: &Permissions,
  ) -> io::Result<()>;

  /// Changes the timestamps of the file or directory at the given path.
  fn set_times(
    &self,
    path: &Path,
    times: &FileTimes,
  ) -> io::Result<()>;

  fn symlink(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()>;

  fn symlink_metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>>;

  /// Returns `Ok(true)` if the path exists.
  fn try_exists(
    &self,
    path: &Path,
  ) -> io::Result<bool>;

  /// Returns `true` if the path exists, ignoring any errors.
  fn exists(
    &self,
    path: &Path,
  ) -> bool {
    self.try_exists(path).unwrap_or(false)
  }

  fn write(
    &self,
    path: &Path,
    contents: &[u8],
  ) -> io::Result<()>;
}

pub trait Metadata {
  // TODO(deferred): expose Unix extended metadata (`dev`, `ino`, `uid`, `gid`,
  // `nlink`, `blocks`) and the additional file-kind predicates
  // (`is_block_device`, `is_char_device`, `is_fifo`, `is_socket`), backed by
  // `std::os::unix::fs::MetadataExt`/`FileTypeExt` on Unix.
  fn accessed(&self) -> io::Result<SystemTime>;
  fn created(&self) -> io::Result<SystemTime>;
  fn file_type(&self) -> Box<dyn FileType + Send + Sync>;
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
  fn modified(&self) -> io::Result<SystemTime>;
  fn permissions(&self) -> Permissions;
  fn len(&self) -> u64;

  /// Returns `true` if the file is empty (`len` is `0`).
  fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

/// The permissions of a file or directory.
///
/// Unlike the standard library this is a single concrete value type, so it can
/// be cloned and passed around without boxing. On Unix the full permission
/// `mode` is available; on other platforms only the readonly flag is tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
  readonly: bool,
  #[cfg(unix)]
  mode: u32,
}

impl Permissions {
  /// Creates a new, writable permission set.
  #[must_use]
  pub fn new() -> Self {
    Self {
      readonly: false,
      #[cfg(unix)]
      mode: 0o666,
    }
  }

  /// Creates a permission set from a readonly flag.
  #[must_use]
  pub fn from_readonly(readonly: bool) -> Self {
    let mut permissions = Self::new();
    permissions.set_readonly(readonly);
    permissions
  }

  /// Creates a permission set from Unix permission bits.
  #[cfg(unix)]
  #[must_use]
  pub fn from_mode(mode: u32) -> Self {
    Self {
      readonly: mode & 0o222 == 0,
      mode,
    }
  }

  /// Returns `true` if the file is readonly.
  #[must_use]
  pub fn readonly(&self) -> bool {
    self.readonly
  }

  /// Sets the readonly flag.
  ///
  /// On Unix this toggles the write bits of the mode.
  pub fn set_readonly(
    &mut self,
    readonly: bool,
  ) {
    self.readonly = readonly;
    #[cfg(unix)]
    {
      if readonly {
        self.mode &= !0o222;
      } else {
        self.mode |= 0o222;
      }
    }
  }

  /// Returns the Unix permission bits.
  #[cfg(unix)]
  #[must_use]
  pub fn mode(&self) -> u32 {
    self.mode
  }

  /// Sets the Unix permission bits.
  #[cfg(unix)]
  pub fn set_mode(
    &mut self,
    mode: u32,
  ) {
    self.mode = mode;
    self.readonly = mode & 0o222 == 0;
  }
}

impl Default for Permissions {
  fn default() -> Self {
    Self::new()
  }
}

/// Value used to set the accessed and modified timestamps of a file.
///
/// Mirrors [`std::fs::FileTimes`] without leaking the standard library type
/// into the abstraction.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTimes {
  accessed: Option<SystemTime>,
  modified: Option<SystemTime>,
}

impl FileTimes {
  /// Creates a new `FileTimes` with no timestamps set.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns the accessed timestamp, if set.
  #[must_use]
  pub fn accessed(&self) -> Option<SystemTime> {
    self.accessed
  }

  /// Returns the modified timestamp, if set.
  #[must_use]
  pub fn modified(&self) -> Option<SystemTime> {
    self.modified
  }

  /// Sets the accessed timestamp.
  pub fn set_accessed(
    &mut self,
    time: SystemTime,
  ) -> &mut Self {
    self.accessed = Some(time);
    self
  }

  /// Sets the modified timestamp.
  pub fn set_modified(
    &mut self,
    time: SystemTime,
  ) -> &mut Self {
    self.modified = Some(time);
    self
  }
}

/// Options and flags which can be used to configure how a file is opened.
///
/// Mirrors [`std::fs::OpenOptions`] without leaking the standard library type
/// into the abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenOptions {
  read: bool,
  write: bool,
  append: bool,
  truncate: bool,
  create: bool,
  create_new: bool,
}

impl OpenOptions {
  /// Creates a blank new set of options.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the option for read access.
  pub fn read(
    &mut self,
    read: bool,
  ) -> &mut Self {
    self.read = read;
    self
  }

  /// Sets the option for write access.
  pub fn write(
    &mut self,
    write: bool,
  ) -> &mut Self {
    self.write = write;
    self
  }

  /// Sets the option for append mode.
  pub fn append(
    &mut self,
    append: bool,
  ) -> &mut Self {
    self.append = append;
    self
  }

  /// Sets the option to truncate the file.
  pub fn truncate(
    &mut self,
    truncate: bool,
  ) -> &mut Self {
    self.truncate = truncate;
    self
  }

  /// Sets the option to create the file if it does not exist.
  pub fn create(
    &mut self,
    create: bool,
  ) -> &mut Self {
    self.create = create;
    self
  }

  /// Sets the option to create a new file, failing if it already exists.
  pub fn create_new(
    &mut self,
    create_new: bool,
  ) -> &mut Self {
    self.create_new = create_new;
    self
  }

  /// Returns whether read access is enabled.
  #[must_use]
  pub fn get_read(&self) -> bool {
    self.read
  }

  /// Returns whether write access is enabled.
  #[must_use]
  pub fn get_write(&self) -> bool {
    self.write
  }

  /// Returns whether append mode is enabled.
  #[must_use]
  pub fn get_append(&self) -> bool {
    self.append
  }

  /// Returns whether truncation is enabled.
  #[must_use]
  pub fn get_truncate(&self) -> bool {
    self.truncate
  }

  /// Returns whether creating the file is enabled.
  #[must_use]
  pub fn get_create(&self) -> bool {
    self.create
  }

  /// Returns whether creating a new file is enabled.
  #[must_use]
  pub fn get_create_new(&self) -> bool {
    self.create_new
  }
}

/// A builder used to create directories in various manners.
///
/// Mirrors [`std::fs::DirBuilder`] without leaking the standard library type
/// into the abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirBuilder {
  recursive: bool,
  #[cfg(unix)]
  mode: u32,
  #[cfg(unix)]
  has_mode: bool,
}

impl DirBuilder {
  /// Creates a new builder with default options.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets whether intermediate directories should be created.
  pub fn recursive(
    &mut self,
    recursive: bool,
  ) -> &mut Self {
    self.recursive = recursive;
    self
  }

  /// Sets the Unix permission mode for the created directories.
  #[cfg(unix)]
  pub fn mode(
    &mut self,
    mode: u32,
  ) -> &mut Self {
    self.mode = mode;
    self.has_mode = true;
    self
  }

  /// Returns whether intermediate directories will be created.
  #[must_use]
  pub fn get_recursive(&self) -> bool {
    self.recursive
  }

  /// Returns the Unix permission mode, if one was set.
  #[cfg(unix)]
  #[must_use]
  pub fn get_mode(&self) -> Option<u32> {
    if self.has_mode { Some(self.mode) } else { None }
  }
}

pub trait FileType {
  // TODO(deferred): add `is_block_device`, `is_char_device`, `is_fifo` and
  // `is_socket`, backed by `std::os::unix::fs::FileTypeExt` on Unix.
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
}

/// An entry within a directory, as returned by [`FileSystem::read_dir`].
pub trait DirEntry {
  /// Full path to this entry.
  fn path(&self) -> PathBuf;

  /// The final component of the path of this entry.
  fn file_name(&self) -> OsString;

  /// The file type of this entry, without following symlinks.
  fn file_type(&self) -> io::Result<Box<dyn FileType + Send + Sync>>;

  /// Metadata for this entry, without following symlinks.
  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>>;
}

/// An abstract iterator over the entries of a directory.
///
/// This is implemented by any iterator yielding boxed [`DirEntry`] values,
/// allowing backends to return their own concrete iterators.
pub trait ReadDir: Iterator<Item = io::Result<Box<dyn DirEntry + Send + Sync>>> {}

impl<T> ReadDir for T where T: Iterator<Item = io::Result<Box<dyn DirEntry + Send + Sync>>> {}

/// A seekable, read/write file handle.
///
/// Its read / write / seek supertraits are `std::io`'s own [`io::Read`] /
/// [`io::Write`] / [`io::Seek`], and it requires `Send + Sync` so handles can
/// be moved and shared across threads. The trait stays object-safe, so handles
/// are handed out as `Box<dyn File>`.
pub trait File: io::Read + io::Seek + io::Write + Send + Sync {
  /// Queries metadata about the underlying file.
  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>>;

  /// Truncates or extends the underlying file, updating the size to `size`.
  fn set_len(
    &self,
    size: u64,
  ) -> io::Result<()>;

  /// Changes the permissions of the underlying file.
  fn set_permissions(
    &self,
    perm: &Permissions,
  ) -> io::Result<()>;

  /// Changes the timestamps of the underlying file.
  fn set_times(
    &self,
    times: &FileTimes,
  ) -> io::Result<()>;

  /// Attempts to sync all OS-internal metadata to disk.
  fn sync_all(&self) -> io::Result<()>;

  /// Attempts to sync file data, but not metadata, to disk.
  fn sync_data(&self) -> io::Result<()>;
}
