use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::OsFileType;
use super::OsMetadata;
use crate::DirEntry;
use crate::FileType;
use crate::Metadata;

pub struct OsDirEntry(fs::DirEntry);

impl From<fs::DirEntry> for OsDirEntry {
  fn from(value: fs::DirEntry) -> Self {
    Self(value)
  }
}

impl DirEntry for OsDirEntry {
  fn path(&self) -> PathBuf {
    self.0.path()
  }

  fn file_name(&self) -> OsString {
    self.0.file_name()
  }

  fn file_type(&self) -> io::Result<Box<dyn FileType + Send + Sync>> {
    let file_type = self.0.file_type()?;
    Ok(Box::new(OsFileType::from(file_type)))
  }

  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let metadata = self.0.metadata()?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }
}
