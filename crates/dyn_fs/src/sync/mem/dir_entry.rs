use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use super::MemoryMetadata;
use crate::DirEntry;
use crate::FileType;
use crate::Metadata;

/// A directory entry of the in-memory file system.
#[derive(Debug, Clone)]
pub struct MemoryDirEntry {
  pub(crate) path: PathBuf,
  pub(crate) metadata: MemoryMetadata,
}

impl DirEntry for MemoryDirEntry {
  fn path(&self) -> PathBuf {
    self.path.clone()
  }

  fn file_name(&self) -> OsString {
    self
      .path
      .file_name()
      .map(std::ffi::OsStr::to_os_string)
      .unwrap_or_default()
  }

  fn file_type(&self) -> io::Result<Box<dyn FileType + Send + Sync>> {
    Ok(Box::new(self.metadata.file_type))
  }

  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    Ok(Box::new(self.metadata.clone()))
  }
}
