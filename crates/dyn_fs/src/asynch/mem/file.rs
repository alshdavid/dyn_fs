use std::collections::HashMap;
use std::io;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use std::task::Context;
use std::task::Poll;
use std::time::SystemTime;

use super::node::AsyncNode;
use super::node::AsyncNodeKind;
use crate::asynch::file_system::AsyncFile;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::AsyncRead;
use crate::asynch::file_system::AsyncSeek;
use crate::asynch::file_system::AsyncWrite;
use crate::asynch::file_system::BoxFuture;
use crate::asynch::file_system::FileTimes;
use crate::asynch::file_system::Permissions;

/// A handle onto a file within an [`super::AsyncMemoryFileSystem`].
///
/// Reads and writes operate directly on the shared node table, so changes are
/// visible to the file system immediately, and to any hard links of the file.
///
/// Every operation completes synchronously, so the poll-based [`AsyncRead`],
/// [`AsyncWrite`] and [`AsyncSeek`] methods always return [`Poll::Ready`] and
/// never yield.
pub struct AsyncMemoryFile {
  pub(crate) files: Arc<RwLock<HashMap<PathBuf, AsyncNode>>>,
  pub(crate) path: PathBuf,
  pub(crate) position: u64,
  pub(crate) append: bool,
}

impl AsyncMemoryFile {
  fn poisoned() -> io::Error {
    io::Error::other("memory file system lock poisoned")
  }

  /// Returns the shared, mutable contents of the underlying file.
  fn data(&self) -> io::Result<Arc<RwLock<Vec<u8>>>> {
    let files = self.files.read().map_err(|_| Self::poisoned())?;
    match files.get(&self.path) {
      Some(AsyncNode {
        kind: AsyncNodeKind::File(data),
        ..
      }) => Ok(Arc::clone(data)),
      Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
      None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
    }
  }

  /// Returns the length of the underlying file in bytes.
  fn length(&self) -> io::Result<u64> {
    let data = self.data()?;
    let data = data.read().map_err(|_| Self::poisoned())?;
    Ok(data.len() as u64)
  }

  /// Updates the modified timestamp of the underlying file.
  fn touch(&self) -> io::Result<()> {
    let mut files = self.files.write().map_err(|_| Self::poisoned())?;
    if let Some(node) = files.get_mut(&self.path) {
      node.modified = SystemTime::now();
    }
    Ok(())
  }
}

impl AsyncRead for AsyncMemoryFile {
  fn poll_read(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &mut ::tokio::io::ReadBuf<'_>,
  ) -> Poll<io::Result<()>> {
    let this = self.get_mut();

    let data = match this.data() {
      Ok(data) => data,
      Err(error) => return Poll::Ready(Err(error)),
    };
    let data = match data.read() {
      Ok(data) => data,
      Err(_) => return Poll::Ready(Err(Self::poisoned())),
    };

    let start = this.position as usize;
    if start < data.len() {
      let end = std::cmp::min(start + buf.remaining(), data.len());
      buf.put_slice(&data[start..end]);
      this.position = end as u64;
    }

    Poll::Ready(Ok(()))
  }
}

impl AsyncWrite for AsyncMemoryFile {
  fn poll_write(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    let this = self.get_mut();

    let data = match this.data() {
      Ok(data) => data,
      Err(error) => return Poll::Ready(Err(error)),
    };

    {
      let mut data = match data.write() {
        Ok(data) => data,
        Err(_) => return Poll::Ready(Err(Self::poisoned())),
      };

      if this.append {
        this.position = data.len() as u64;
      }

      let start = this.position as usize;
      if start > data.len() {
        data.resize(start, 0);
      }

      let end = start + buf.len();
      if end > data.len() {
        data.resize(end, 0);
      }

      data[start..end].copy_from_slice(buf);
      this.position = end as u64;
    }

    if let Err(error) = this.touch() {
      return Poll::Ready(Err(error));
    }

    Poll::Ready(Ok(buf.len()))
  }

  fn poll_flush(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(Ok(()))
  }

  fn poll_shutdown(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<()>> {
    Poll::Ready(Ok(()))
  }
}

impl AsyncSeek for AsyncMemoryFile {
  fn start_seek(
    self: Pin<&mut Self>,
    position: SeekFrom,
  ) -> io::Result<()> {
    let this = self.get_mut();

    let current = this.position as i64;
    let length = this.length()? as i64;

    let next = match position {
      SeekFrom::Start(offset) => offset as i64,
      SeekFrom::Current(offset) => current + offset,
      SeekFrom::End(offset) => length + offset,
    };

    if next < 0 {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid seek to a negative position",
      ));
    }

    this.position = next as u64;

    Ok(())
  }

  fn poll_complete(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<u64>> {
    Poll::Ready(Ok(self.position))
  }
}

impl AsyncFile for AsyncMemoryFile {
  fn metadata(&self) -> BoxFuture<'_, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let files = self.files.read().map_err(|_| Self::poisoned())?;
      let node = files
        .get(&self.path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))?;
      Ok(Box::new(node.metadata()) as Box<dyn AsyncMetadata + Send + Sync>)
    })
  }

  fn set_len(
    &self,
    size: u64,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move {
      let data = self.data()?;
      data
        .write()
        .map_err(|_| Self::poisoned())?
        .resize(size as usize, 0);
      self.touch()?;
      Ok(())
    })
  }

  fn set_permissions(
    &self,
    perm: Permissions,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move {
      let mut files = self.files.write().map_err(|_| Self::poisoned())?;
      match files.get_mut(&self.path) {
        Some(node) => {
          node.permissions = perm;
          Ok(())
        }
        None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
      }
    })
  }

  fn set_times(
    &self,
    times: FileTimes,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move {
      let mut files = self.files.write().map_err(|_| Self::poisoned())?;
      match files.get_mut(&self.path) {
        Some(node) => {
          if let Some(accessed) = times.accessed() {
            node.accessed = accessed;
          }
          if let Some(modified) = times.modified() {
            node.modified = modified;
          }
          Ok(())
        }
        None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
      }
    })
  }

  fn sync_all(&self) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Ok(()) })
  }

  fn sync_data(&self) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Ok(()) })
  }
}
