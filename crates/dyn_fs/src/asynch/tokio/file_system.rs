use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use super::TokioMetadata;
use super::TokioReadDir;
use crate::asynch::file_system::AsyncFile;
use crate::asynch::file_system::AsyncFileSystem;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::AsyncRead;
use crate::asynch::file_system::AsyncReadDir;
use crate::asynch::file_system::AsyncSeek;
use crate::asynch::file_system::AsyncWrite;
use crate::asynch::file_system::BoxFuture;
use crate::asynch::file_system::DirBuilder;
use crate::asynch::file_system::FileTimes;
use crate::asynch::file_system::OpenOptions;
use crate::asynch::file_system::Permissions;

/// An [`AsyncFileSystem`] backed by `tokio`'s asynchronous filesystem API.
#[derive(Default, Debug, Clone)]
pub struct TokioFileSystem;

impl TokioFileSystem {
  /// Creates a file system backed by `tokio`'s asynchronous filesystem API,
  /// returned as a shared [`Arc`] trait object.
  #[allow(clippy::new_ret_no_self)]
  pub fn new() -> Arc<dyn AsyncFileSystem> {
    Arc::new(Self)
  }
}

impl AsyncFileSystem for TokioFileSystem {
  fn open_with<'a>(
    &'a self,
    path: &'a Path,
    options: OpenOptions,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncFile + Send + Sync>>> {
    Box::pin(async move {
      let mut open_options = ::tokio::fs::OpenOptions::new();
      open_options
        .read(options.get_read())
        .write(options.get_write())
        .append(options.get_append())
        .truncate(options.get_truncate())
        .create(options.get_create())
        .create_new(options.get_create_new());

      let file = open_options.open(path).await?;

      Ok(Box::new(TokioFile {
        file,
        path: path.to_path_buf(),
      }) as Box<dyn AsyncFile + Send + Sync>)
    })
  }

  fn canonicalize<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>> {
    Box::pin(async move { ::tokio::fs::canonicalize(path).await })
  }

  fn read_link<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>> {
    Box::pin(async move { ::tokio::fs::read_link(path).await })
  }

  fn copy<'a>(
    &'a self,
    from: &'a Path,
    to: &'a Path,
  ) -> BoxFuture<'a, io::Result<u64>> {
    Box::pin(async move { ::tokio::fs::copy(from, to).await })
  }

  fn create_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::create_dir(path).await })
  }

  fn create_dir_all<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::create_dir_all(path).await })
  }

  fn create_dir_with<'a>(
    &'a self,
    path: &'a Path,
    builder: DirBuilder,
  ) -> BoxFuture<'a, io::Result<()>> {
    let path = path.to_path_buf();
    Box::pin(async move {
      ::tokio::task::spawn_blocking(move || {
        let mut dir_builder = fs::DirBuilder::new();
        dir_builder.recursive(builder.get_recursive());
        #[cfg(unix)]
        if let Some(mode) = builder.get_mode() {
          dir_builder.mode(mode);
        }
        dir_builder.create(&path)
      })
      .await
      .map_err(join_error)?
    })
  }

  fn hard_link<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::hard_link(original, link).await })
  }

  fn metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let metadata = ::tokio::fs::metadata(path).await?;
      Ok(Box::new(TokioMetadata::from(metadata)) as Box<dyn AsyncMetadata + Send + Sync>)
    })
  }

  fn read<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Vec<u8>>> {
    Box::pin(async move { ::tokio::fs::read(path).await })
  }

  fn read_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncReadDir>>> {
    let path = path.to_path_buf();
    Box::pin(async move {
      // `tokio`'s `DirEntry` resolves metadata asynchronously, whereas the
      // `AsyncDirEntry` trait exposes it synchronously. Read the directory on a
      // blocking thread so the entries can answer synchronously.
      let entries = ::tokio::task::spawn_blocking(move || {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&path)? {
          entries.push(entry?);
        }
        Ok::<_, io::Error>(entries)
      })
      .await
      .map_err(join_error)??;

      Ok(Box::new(TokioReadDir::from(entries)) as Box<dyn AsyncReadDir>)
    })
  }

  fn read_to_string<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<String>> {
    Box::pin(async move { ::tokio::fs::read_to_string(path).await })
  }

  fn remove_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::remove_dir(path).await })
  }

  fn remove_dir_all<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::remove_dir_all(path).await })
  }

  fn remove_file<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::remove_file(path).await })
  }

  fn rename<'a>(
    &'a self,
    from: &'a Path,
    to: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::rename(from, to).await })
  }

  fn set_permissions<'a>(
    &'a self,
    path: &'a Path,
    perm: Permissions,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      #[cfg(unix)]
      {
        let permissions = fs::Permissions::from_mode(perm.mode());
        ::tokio::fs::set_permissions(path, permissions).await
      }
      #[cfg(not(unix))]
      {
        let mut permissions = ::tokio::fs::metadata(path).await?.permissions();
        permissions.set_readonly(perm.readonly());
        ::tokio::fs::set_permissions(path, permissions).await
      }
    })
  }

  fn set_times<'a>(
    &'a self,
    path: &'a Path,
    times: FileTimes,
  ) -> BoxFuture<'a, io::Result<()>> {
    let path = path.to_path_buf();
    Box::pin(async move {
      ::tokio::task::spawn_blocking(move || {
        let file = fs::File::open(&path)?;
        file.set_times(to_std_file_times(&times))
      })
      .await
      .map_err(join_error)?
    })
  }

  #[cfg(unix)]
  fn symlink<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::symlink(original, link).await })
  }

  #[cfg(windows)]
  fn symlink<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let is_dir = ::tokio::fs::metadata(original)
        .await
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);

      if is_dir {
        ::tokio::fs::symlink_dir(original, link).await
      } else {
        ::tokio::fs::symlink_file(original, link).await
      }
    })
  }

  fn symlink_metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let metadata = ::tokio::fs::symlink_metadata(path).await?;
      Ok(Box::new(TokioMetadata::from(metadata)) as Box<dyn AsyncMetadata + Send + Sync>)
    })
  }

  fn try_exists<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<bool>> {
    Box::pin(async move { ::tokio::fs::try_exists(path).await })
  }

  fn write<'a>(
    &'a self,
    path: &'a Path,
    contents: &'a [u8],
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move { ::tokio::fs::write(path, contents).await })
  }
}

/// An asynchronous file handle backed by `tokio::fs::File`.
///
/// The originating path is retained so operations `tokio` does not expose (such
/// as setting timestamps) can be performed on a blocking thread.
pub struct TokioFile {
  file: ::tokio::fs::File,
  path: PathBuf,
}

impl AsyncRead for TokioFile {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ::tokio::io::ReadBuf<'_>,
  ) -> Poll<io::Result<()>> {
    Pin::new(&mut self.get_mut().file).poll_read(cx, buf)
  }
}

impl AsyncWrite for TokioFile {
  fn poll_write(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    Pin::new(&mut self.get_mut().file).poll_write(cx, buf)
  }

  fn poll_flush(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Pin::new(&mut self.get_mut().file).poll_flush(cx)
  }

  fn poll_shutdown(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Pin::new(&mut self.get_mut().file).poll_shutdown(cx)
  }
}

impl AsyncSeek for TokioFile {
  fn start_seek(
    self: Pin<&mut Self>,
    position: io::SeekFrom,
  ) -> io::Result<()> {
    Pin::new(&mut self.get_mut().file).start_seek(position)
  }

  fn poll_complete(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<io::Result<u64>> {
    Pin::new(&mut self.get_mut().file).poll_complete(cx)
  }
}

impl AsyncFile for TokioFile {
  fn metadata(&self) -> BoxFuture<'_, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let metadata = self.file.metadata().await?;
      Ok(Box::new(TokioMetadata::from(metadata)) as Box<dyn AsyncMetadata + Send + Sync>)
    })
  }

  fn set_len(
    &self,
    size: u64,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { self.file.set_len(size).await })
  }

  fn set_permissions(
    &self,
    perm: Permissions,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move {
      #[cfg(unix)]
      {
        let permissions = fs::Permissions::from_mode(perm.mode());
        self.file.set_permissions(permissions).await
      }
      #[cfg(not(unix))]
      {
        let mut permissions = self.file.metadata().await?.permissions();
        permissions.set_readonly(perm.readonly());
        self.file.set_permissions(permissions).await
      }
    })
  }

  fn set_times(
    &self,
    times: FileTimes,
  ) -> BoxFuture<'_, io::Result<()>> {
    let path = self.path.clone();
    Box::pin(async move {
      ::tokio::task::spawn_blocking(move || {
        let file = fs::File::open(&path)?;
        file.set_times(to_std_file_times(&times))
      })
      .await
      .map_err(join_error)?
    })
  }

  fn sync_all(&self) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { self.file.sync_all().await })
  }

  fn sync_data(&self) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { self.file.sync_data().await })
  }
}

fn join_error(error: ::tokio::task::JoinError) -> io::Error {
  io::Error::other(error)
}

fn to_std_file_times(times: &FileTimes) -> fs::FileTimes {
  let mut file_times = fs::FileTimes::new();
  if let Some(accessed) = times.accessed() {
    file_times = file_times.set_accessed(accessed);
  }
  if let Some(modified) = times.modified() {
    file_times = file_times.set_modified(modified);
  }
  file_times
}
