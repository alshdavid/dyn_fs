//! Asynchronous abstraction of the file system
//!
//! This module defines [`AsyncFileSystem`] — an object-safe, runtime-agnostic
//! virtual file system interface — together with the value types and auxiliary
//! traits it exposes ([`AsyncMetadata`], [`AsyncFileType`], [`AsyncDirEntry`],
//! [`AsyncReadDir`], [`AsyncFile`]).
//!
//! Every fallible operation returns a [`BoxFuture`], and file handles implement
//! `tokio`'s [`AsyncRead`]/[`AsyncWrite`]/[`AsyncSeek`], so the trait can be
//! backed by both the operating system and an in-memory implementation and be
//! driven from any executor.

use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::SystemTime;

pub use tokio::io::AsyncRead;
pub use tokio::io::AsyncSeek;
pub use tokio::io::AsyncWrite;

/// A boxed, `Send` future — the return type of the fallible methods on
/// [`AsyncFileSystem`] and [`AsyncFile`].
///
/// Requiring `Send` lets a `dyn AsyncFileSystem` be shared across a
/// work-stealing executor such as `tokio`'s multi-threaded runtime. Path and
/// slice arguments are borrowed for the lifetime of the future, so a backend
/// that can complete without spawning avoids any defensive copying.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A dynamically-dispatched, asynchronous virtual file system.
///
/// The trait is **object-safe** so it can be used as `&dyn AsyncFileSystem` or
/// `Arc<dyn AsyncFileSystem>`: methods return [`BoxFuture`] rather than using
/// `async fn`, which would not be dyn-compatible. It is also **runtime
/// agnostic** — no executor or runtime is baked in.
///
/// The configuration value types ([`AsyncOpenOptions`], [`AsyncDirBuilder`],
/// [`AsyncPermissions`], [`AsyncFileTimes`]) are `Copy` and passed **by value**
/// so the returned futures never borrow them.
pub trait AsyncFileSystem: Send + Sync {
  /// Opens a file in read-only mode.
  fn open<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncFile + Send + Sync>>> {
    let mut options = AsyncOpenOptions::new();
    options.read(true);
    self.open_with(path, options)
  }

  /// Opens a file with the given options.
  fn open_with<'a>(
    &'a self,
    path: &'a Path,
    options: AsyncOpenOptions,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncFile + Send + Sync>>>;

  fn canonicalize<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>>;

  fn read_link<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>>;

  fn copy<'a>(
    &'a self,
    from: &'a Path,
    to: &'a Path,
  ) -> BoxFuture<'a, io::Result<u64>>;

  fn create_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn create_dir_all<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  /// Creates a directory using the given builder.
  fn create_dir_with<'a>(
    &'a self,
    path: &'a Path,
    builder: AsyncDirBuilder,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn hard_link<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>>;

  fn read<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Vec<u8>>>;

  fn read_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncReadDir>>>;

  fn read_to_string<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<String>>;

  fn remove_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn remove_dir_all<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn remove_file<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn rename<'a>(
    &'a self,
    from: &'a Path,
    to: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn set_permissions<'a>(
    &'a self,
    path: &'a Path,
    perm: AsyncPermissions,
  ) -> BoxFuture<'a, io::Result<()>>;

  /// Changes the timestamps of the file or directory at the given path.
  fn set_times<'a>(
    &'a self,
    path: &'a Path,
    times: AsyncFileTimes,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn symlink<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>>;

  fn symlink_metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>>;

  /// Returns `Ok(true)` if the path exists.
  fn try_exists<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<bool>>;

  /// Returns `true` if the path exists, ignoring any errors.
  fn exists<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, bool> {
    Box::pin(async move { self.try_exists(path).await.unwrap_or(false) })
  }

  fn write<'a>(
    &'a self,
    path: &'a Path,
    contents: &'a [u8],
  ) -> BoxFuture<'a, io::Result<()>>;
}

pub trait AsyncMetadata {
  // TODO(deferred): expose Unix extended metadata (`dev`, `ino`, `uid`, `gid`,
  // `nlink`, `blocks`) and the additional file-kind predicates
  // (`is_block_device`, `is_char_device`, `is_fifo`, `is_socket`), backed by
  // `std::os::unix::fs::MetadataExt`/`FileTypeExt` on Unix.
  fn accessed(&self) -> io::Result<SystemTime>;
  fn created(&self) -> io::Result<SystemTime>;
  fn file_type(&self) -> Box<dyn AsyncFileType + Send + Sync>;
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
  fn modified(&self) -> io::Result<SystemTime>;
  fn permissions(&self) -> AsyncPermissions;
  fn len(&self) -> u64;
}

/// The permissions of a file or directory.
///
/// Unlike the standard library this is a single concrete value type, so it can
/// be cloned and passed around without boxing. On Unix the full permission
/// `mode` is available; on other platforms only the readonly flag is tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsyncPermissions {
  readonly: bool,
  #[cfg(unix)]
  mode: u32,
}

impl AsyncPermissions {
  /// Creates a new, writable permission set.
  pub fn new() -> Self {
    Self {
      readonly: false,
      #[cfg(unix)]
      mode: 0o666,
    }
  }

  /// Creates a permission set from a readonly flag.
  pub fn from_readonly(readonly: bool) -> Self {
    let mut permissions = Self::new();
    permissions.set_readonly(readonly);
    permissions
  }

  /// Creates a permission set from Unix permission bits.
  #[cfg(unix)]
  pub fn from_mode(mode: u32) -> Self {
    Self {
      readonly: mode & 0o222 == 0,
      mode,
    }
  }

  /// Returns `true` if the file is readonly.
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

impl Default for AsyncPermissions {
  fn default() -> Self {
    Self::new()
  }
}

/// Value used to set the accessed and modified timestamps of a file.
///
/// Mirrors [`std::fs::FileTimes`] without leaking the standard library type
/// into the abstraction.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsyncFileTimes {
  accessed: Option<SystemTime>,
  modified: Option<SystemTime>,
}

impl AsyncFileTimes {
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

/// Options and flags which can be used to configure how a file is opened.
///
/// Mirrors [`std::fs::OpenOptions`] without leaking the standard library type
/// into the abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsyncOpenOptions {
  read: bool,
  write: bool,
  append: bool,
  truncate: bool,
  create: bool,
  create_new: bool,
}

impl AsyncOpenOptions {
  /// Creates a blank new set of options.
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
  pub fn get_read(&self) -> bool {
    self.read
  }

  /// Returns whether write access is enabled.
  pub fn get_write(&self) -> bool {
    self.write
  }

  /// Returns whether append mode is enabled.
  pub fn get_append(&self) -> bool {
    self.append
  }

  /// Returns whether truncation is enabled.
  pub fn get_truncate(&self) -> bool {
    self.truncate
  }

  /// Returns whether creating the file is enabled.
  pub fn get_create(&self) -> bool {
    self.create
  }

  /// Returns whether creating a new file is enabled.
  pub fn get_create_new(&self) -> bool {
    self.create_new
  }
}

/// A builder used to create directories in various manners.
///
/// Mirrors [`std::fs::DirBuilder`] without leaking the standard library type
/// into the abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsyncDirBuilder {
  recursive: bool,
  #[cfg(unix)]
  mode: u32,
  #[cfg(unix)]
  has_mode: bool,
}

impl AsyncDirBuilder {
  /// Creates a new builder with default options.
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
  pub fn get_recursive(&self) -> bool {
    self.recursive
  }

  /// Returns the Unix permission mode, if one was set.
  #[cfg(unix)]
  pub fn get_mode(&self) -> Option<u32> {
    if self.has_mode { Some(self.mode) } else { None }
  }
}

pub trait AsyncFileType {
  // TODO(deferred): add `is_block_device`, `is_char_device`, `is_fifo` and
  // `is_socket`, backed by `std::os::unix::fs::FileTypeExt` on Unix.
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
}

/// An entry within a directory, as returned by [`AsyncFileSystem::read_dir`].
pub trait AsyncDirEntry {
  /// Full path to this entry.
  fn path(&self) -> PathBuf;

  /// The final component of the path of this entry.
  fn file_name(&self) -> OsString;

  /// The file type of this entry, without following symlinks.
  fn file_type(&self) -> io::Result<Box<dyn AsyncFileType + Send + Sync>>;

  /// Metadata for this entry, without following symlinks.
  fn metadata(&self) -> io::Result<Box<dyn AsyncMetadata + Send + Sync>>;
}

/// An asynchronous, streaming list of directory entries.
///
/// Mirrors `tokio::fs::ReadDir`: call [`AsyncReadDir::next_entry`] repeatedly
/// until it resolves to `Ok(None)`. Returning a stream rather than a `Vec` lets
/// a backend that reads a directory lazily avoid buffering every entry up
/// front.
pub trait AsyncReadDir: Send {
  /// Resolves to the next entry, or `None` once the directory is exhausted.
  fn next_entry(
    &mut self
  ) -> BoxFuture<'_, io::Result<Option<Box<dyn AsyncDirEntry + Send + Sync>>>>;
}

/// An asynchronous, seekable file handle.
///
/// Its read / write / seek supertraits are `tokio`'s own [`AsyncRead`] /
/// [`AsyncWrite`] / [`AsyncSeek`] (re-exported here), so a handle plugs
/// directly into `tokio`'s combinators. The trait stays object-safe, so handles
/// are handed out as `Box<dyn AsyncFile>`.
pub trait AsyncFile: AsyncRead + AsyncWrite + AsyncSeek + Send + Sync + Unpin {
  /// Queries metadata about the underlying file.
  fn metadata(&self) -> BoxFuture<'_, io::Result<Box<dyn AsyncMetadata + Send + Sync>>>;

  /// Truncates or extends the underlying file, updating the size to `size`.
  fn set_len(
    &self,
    size: u64,
  ) -> BoxFuture<'_, io::Result<()>>;

  /// Changes the permissions of the underlying file.
  fn set_permissions(
    &self,
    perm: AsyncPermissions,
  ) -> BoxFuture<'_, io::Result<()>>;

  /// Changes the timestamps of the underlying file.
  fn set_times(
    &self,
    times: AsyncFileTimes,
  ) -> BoxFuture<'_, io::Result<()>>;

  /// Attempts to sync all OS-internal metadata to disk.
  fn sync_all(&self) -> BoxFuture<'_, io::Result<()>>;

  /// Attempts to sync file data, but not metadata, to disk.
  fn sync_data(&self) -> BoxFuture<'_, io::Result<()>>;
}
