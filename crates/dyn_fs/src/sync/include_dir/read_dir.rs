use std::io;

use super::IncludeDirDirEntry;
use crate::DirEntry;

/// An iterator over the entries of an embedded `include_dir` directory.
pub struct IncludeDirReadDir(std::vec::IntoIter<IncludeDirDirEntry>);

impl From<Vec<IncludeDirDirEntry>> for IncludeDirReadDir {
  fn from(entries: Vec<IncludeDirDirEntry>) -> Self {
    Self(entries.into_iter())
  }
}

impl Iterator for IncludeDirReadDir {
  type Item = io::Result<Box<dyn DirEntry + Send + Sync>>;

  fn next(&mut self) -> Option<Self::Item> {
    self
      .0
      .next()
      .map(|entry| Ok(Box::new(entry) as Box<dyn DirEntry + Send + Sync>))
  }
}
