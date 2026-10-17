use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::AsyncOsFileType;
use super::AsyncOsMetadata;
use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncFileType;
use crate::asynch::file_system::AsyncMetadata;

pub struct AsyncOsDirEntry(fs::DirEntry);

impl From<fs::DirEntry> for AsyncOsDirEntry {
  fn from(value: fs::DirEntry) -> Self {
    Self(value)
  }
}

impl AsyncDirEntry for AsyncOsDirEntry {
  fn path(&self) -> PathBuf {
    self.0.path()
  }

  fn file_name(&self) -> OsString {
    self.0.file_name()
  }

  fn file_type(&self) -> io::Result<Box<dyn AsyncFileType + Send + Sync>> {
    let file_type = self.0.file_type()?;
    Ok(Box::new(AsyncOsFileType::from(file_type)))
  }

  fn metadata(&self) -> io::Result<Box<dyn AsyncMetadata + Send + Sync>> {
    let metadata = self.0.metadata()?;
    Ok(Box::new(AsyncOsMetadata::from(metadata)))
  }
}
