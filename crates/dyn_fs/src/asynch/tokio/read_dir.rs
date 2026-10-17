use std::fs;
use std::io;

use super::TokioDirEntry;
use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncReadDir;
use crate::asynch::file_system::BoxFuture;

/// An [`AsyncReadDir`] over the entries of a directory that were read eagerly
/// on a blocking thread.
pub struct TokioReadDir(std::vec::IntoIter<fs::DirEntry>);

impl From<Vec<fs::DirEntry>> for TokioReadDir {
  fn from(entries: Vec<fs::DirEntry>) -> Self {
    Self(entries.into_iter())
  }
}

impl AsyncReadDir for TokioReadDir {
  fn next_entry(
    &mut self
  ) -> BoxFuture<'_, io::Result<Option<Box<dyn AsyncDirEntry + Send + Sync>>>> {
    Box::pin(async move {
      match self.0.next() {
        Some(entry) => Ok(Some(
          Box::new(TokioDirEntry::from(entry)) as Box<dyn AsyncDirEntry + Send + Sync>
        )),
        None => Ok(None),
      }
    })
  }
}
