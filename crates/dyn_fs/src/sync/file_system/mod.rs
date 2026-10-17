//! Abstraction of the file system
//!
//! This module contains traits that map the functionality of
//! the Rust standard library FileSystem to trait methods

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

pub trait FileSystem: Send + Sync {
  fn open(
    &self,
    path: &Path,
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
    perm: &(dyn Permissions + Send + Sync),
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

  fn write(
    &self,
    path: &Path,
    contents: &[u8],
  ) -> io::Result<()>;
}

pub trait Metadata {
  fn accessed(&self) -> io::Result<SystemTime>;
  fn created(&self) -> io::Result<SystemTime>;
  fn file_type(&self) -> Box<dyn FileType + Send + Sync>;
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
  fn modified(&self) -> io::Result<SystemTime>;
  fn permissions(&self) -> Box<dyn Permissions + Send + Sync>;
  fn len(&self) -> u64;
}

pub trait Permissions {
  fn readonly(&self) -> bool;

  fn set_readonly(
    &mut self,
    readonly: bool,
  );

  #[cfg(unix)]
  fn mode(&self) -> u32;

  #[cfg(unix)]
  fn set_mode(
    &mut self,
    mode: u32,
  );
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
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns the accessed timestamp, if set.
  pub fn accessed(&self) -> Option<SystemTime> {
    self.accessed
  }

  /// Returns the modified timestamp, if set.
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

pub trait FileType {
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

pub trait File: io::Read + io::Seek + io::Write {
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
    perm: &(dyn Permissions + Send + Sync),
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
