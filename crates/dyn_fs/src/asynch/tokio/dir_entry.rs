use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::TokioFileType;
use super::TokioMetadata;
use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncFileType;
use crate::asynch::file_system::AsyncMetadata;

pub struct TokioDirEntry(fs::DirEntry);

impl From<fs::DirEntry> for TokioDirEntry {
  fn from(value: fs::DirEntry) -> Self {
    Self(value)
  }
}

impl AsyncDirEntry for TokioDirEntry {
  fn path(&self) -> PathBuf {
    self.0.path()
  }

  fn file_name(&self) -> OsString {
    self.0.file_name()
  }

  fn file_type(&self) -> io::Result<Box<dyn AsyncFileType + Send + Sync>> {
    let file_type = self.0.file_type()?;
    Ok(Box::new(TokioFileType::from(file_type)))
  }

  fn metadata(&self) -> io::Result<Box<dyn AsyncMetadata + Send + Sync>> {
    let metadata = self.0.metadata()?;
    Ok(Box::new(TokioMetadata::from(metadata)))
  }
}
