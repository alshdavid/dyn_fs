use std::io;
use std::io::Cursor;
use std::io::Seek;
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use super::AsyncIncludeDirFileType;
use super::AsyncIncludeDirMetadata;
use crate::asynch::file_system::AsyncFile;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::AsyncRead;
use crate::asynch::file_system::AsyncSeek;
use crate::asynch::file_system::AsyncWrite;
use crate::asynch::file_system::BoxFuture;
use crate::asynch::file_system::FileTimes;
use crate::asynch::file_system::Permissions;

/// A read-only asynchronous file handle onto an embedded `include_dir` file.
///
/// Reads and seeks delegate to an in-memory cursor over the file's embedded
/// bytes and always complete on the first poll. Writes always fail with
/// [`io::ErrorKind::Unsupported`] because the embedded contents are read-only.
pub struct AsyncIncludeDirFile {
  pub(crate) cursor: Cursor<&'static [u8]>,
}

fn read_only() -> io::Error {
  io::Error::new(
    io::ErrorKind::Unsupported,
    "include_dir file system is read-only",
  )
}

impl AsyncRead for AsyncIncludeDirFile {
  #[allow(clippy::cast_possible_truncation)]
  fn poll_read(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &mut ::tokio::io::ReadBuf<'_>,
  ) -> Poll<io::Result<()>> {
    let this = self.get_mut();
    let data = this.cursor.get_ref();
    let position = this.cursor.position() as usize;

    if position < data.len() {
      let end = std::cmp::min(position + buf.remaining(), data.len());
      buf.put_slice(&data[position..end]);
      this.cursor.set_position(end as u64);
    }

    Poll::Ready(Ok(()))
  }
}

impl AsyncWrite for AsyncIncludeDirFile {
  fn poll_write(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    _buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    Poll::Ready(Err(read_only()))
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

impl AsyncSeek for AsyncIncludeDirFile {
  fn start_seek(
    self: Pin<&mut Self>,
    position: SeekFrom,
  ) -> io::Result<()> {
    self.get_mut().cursor.seek(position)?;
    Ok(())
  }

  fn poll_complete(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<io::Result<u64>> {
    Poll::Ready(Ok(self.cursor.position()))
  }
}

impl AsyncFile for AsyncIncludeDirFile {
  fn metadata(&self) -> BoxFuture<'_, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      Ok(Box::new(AsyncIncludeDirMetadata {
        file_type: AsyncIncludeDirFileType::file(),
        len: self.cursor.get_ref().len() as u64,
      }) as Box<dyn AsyncMetadata + Send + Sync>)
    })
  }

  fn set_len(
    &self,
    _size: u64,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Err(read_only()) })
  }

  fn set_permissions(
    &self,
    _perm: Permissions,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Err(read_only()) })
  }

  fn set_times(
    &self,
    _times: FileTimes,
  ) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Err(read_only()) })
  }

  fn sync_all(&self) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Ok(()) })
  }

  fn sync_data(&self) -> BoxFuture<'_, io::Result<()>> {
    Box::pin(async move { Ok(()) })
  }
}
