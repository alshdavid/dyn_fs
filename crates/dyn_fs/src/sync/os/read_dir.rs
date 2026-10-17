use std::fs;
use std::io;

use super::OsDirEntry;
use crate::file_system::DirEntry;

pub struct OsReadDir(fs::ReadDir);

impl From<fs::ReadDir> for OsReadDir {
  fn from(value: fs::ReadDir) -> Self {
    Self(value)
  }
}

impl Iterator for OsReadDir {
  type Item = io::Result<Box<dyn DirEntry>>;

  fn next(&mut self) -> Option<Self::Item> {
    match self.0.next()? {
      Ok(entry) => Some(Ok(Box::new(OsDirEntry::from(entry)))),
      Err(error) => Some(Err(error)),
    }
  }
}
