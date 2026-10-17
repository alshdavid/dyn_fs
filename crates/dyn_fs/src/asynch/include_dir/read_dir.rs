use std::io;

use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncReadDir;
use crate::asynch::file_system::BoxFuture;

/// An [`AsyncReadDir`] over a snapshot of an embedded directory's entries.
pub struct AsyncIncludeDirReadDir(std::vec::IntoIter<Box<dyn AsyncDirEntry + Send + Sync>>);

impl From<Vec<Box<dyn AsyncDirEntry + Send + Sync>>> for AsyncIncludeDirReadDir {
  fn from(entries: Vec<Box<dyn AsyncDirEntry + Send + Sync>>) -> Self {
    Self(entries.into_iter())
  }
}

impl AsyncReadDir for AsyncIncludeDirReadDir {
  fn next_entry(
    &mut self
  ) -> BoxFuture<'_, io::Result<Option<Box<dyn AsyncDirEntry + Send + Sync>>>> {
    Box::pin(async move { Ok(self.0.next()) })
  }
}
