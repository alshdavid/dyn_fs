use std::fs;
use std::io;

use super::AsyncOsDirEntry;
use crate::asynch::file_system::AsyncDirEntry;

pub struct AsyncOsReadDir(fs::ReadDir);

impl From<fs::ReadDir> for AsyncOsReadDir {
  fn from(value: fs::ReadDir) -> Self {
    Self(value)
  }
}

impl Iterator for AsyncOsReadDir {
  type Item = io::Result<Box<dyn AsyncDirEntry + Send + Sync>>;

  fn next(&mut self) -> Option<Self::Item> {
    match self.0.next()? {
      Ok(entry) => Some(Ok(Box::new(AsyncOsDirEntry::from(entry)))),
      Err(error) => Some(Err(error)),
    }
  }
}
