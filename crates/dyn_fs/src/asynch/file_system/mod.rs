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
use std::sync::Arc;
use std::time::SystemTime;

pub use ::tokio::io::AsyncRead;
pub use ::tokio::io::AsyncSeek;
pub use ::tokio::io::AsyncWrite;

pub use crate::sync::DirBuilder;
pub use crate::sync::FileTimes;
pub use crate::sync::OpenOptions;
pub use crate::sync::Permissions;

/// A boxed, `Send` future — the return type of the fallible methods on
/// [`AsyncFileSystem`] and [`AsyncFile`].
///
/// Requiring `Send` lets a `dyn AsyncFileSystem` be shared across a
/// work-stealing executor such as `tokio`'s multi-threaded runtime. Path and
/// slice arguments are borrowed for the lifetime of the future, so a backend
/// that can complete without spawning avoids any defensive copying.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type AsyncFileSystemRef = Arc<dyn AsyncFileSystem>;

/// A dynamically-dispatched, asynchronous virtual file system.
///
/// The trait is **object-safe** so it can be used as `&dyn AsyncFileSystem` or
/// `Arc<dyn AsyncFileSystem>`: methods return [`BoxFuture`] rather than using
/// `async fn`, which would not be dyn-compatible. It is also **runtime
/// agnostic** — no executor or runtime is baked in.
///
/// The configuration value types ([`OpenOptions`], [`DirBuilder`],
/// [`Permissions`], [`FileTimes`]) are `Copy` and passed **by value**
/// so the returned futures never borrow them.
pub trait AsyncFileSystem: Send + Sync {
  /// Opens a file in read-only mode.
  fn open<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncFile + Send + Sync>>> {
    let mut options = OpenOptions::new();
    options.read(true);
    self.open_with(path, options)
  }

  /// Opens a file with the given options.
  fn open_with<'a>(
    &'a self,
    path: &'a Path,
    options: OpenOptions,
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
    builder: DirBuilder,
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
    perm: Permissions,
  ) -> BoxFuture<'a, io::Result<()>>;

  /// Changes the timestamps of the file or directory at the given path.
  fn set_times<'a>(
    &'a self,
    path: &'a Path,
    times: FileTimes,
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
  fn permissions(&self) -> Permissions;
  fn len(&self) -> u64;

  /// Returns `true` if the file is empty (`len` is `0`).
  fn is_empty(&self) -> bool {
    self.len() == 0
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
    perm: Permissions,
  ) -> BoxFuture<'_, io::Result<()>>;

  /// Changes the timestamps of the underlying file.
  fn set_times(
    &self,
    times: FileTimes,
  ) -> BoxFuture<'_, io::Result<()>>;

  /// Attempts to sync all OS-internal metadata to disk.
  fn sync_all(&self) -> BoxFuture<'_, io::Result<()>>;

  /// Attempts to sync file data, but not metadata, to disk.
  fn sync_data(&self) -> BoxFuture<'_, io::Result<()>>;
}
