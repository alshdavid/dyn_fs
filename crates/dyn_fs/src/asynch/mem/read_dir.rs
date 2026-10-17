use std::io;

use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncReadDir;
use crate::asynch::file_system::BoxFuture;

/// An [`AsyncReadDir`] over a snapshot of a directory's entries.
///
/// The entries are captured when the directory is read, so iteration never
/// touches the file system lock.
pub struct AsyncMemoryReadDir(std::vec::IntoIter<Box<dyn AsyncDirEntry + Send + Sync>>);

impl From<Vec<Box<dyn AsyncDirEntry + Send + Sync>>> for AsyncMemoryReadDir {
  fn from(entries: Vec<Box<dyn AsyncDirEntry + Send + Sync>>) -> Self {
    Self(entries.into_iter())
  }
}

impl AsyncReadDir for AsyncMemoryReadDir {
  fn next_entry(
    &mut self
  ) -> BoxFuture<'_, io::Result<Option<Box<dyn AsyncDirEntry + Send + Sync>>>> {
    Box::pin(async move { Ok(self.0.next()) })
  }
}
