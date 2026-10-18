use std::io;
use std::time::SystemTime;

use super::MemoryFileType;
use crate::FileType;
use crate::Metadata;
use crate::Permissions;

/// A metadata snapshot of an in-memory file system node.
#[derive(Debug, Clone)]
pub struct MemoryMetadata {
  pub(crate) file_type: MemoryFileType,
  pub(crate) len: u64,
  pub(crate) permissions: Permissions,
  pub(crate) accessed: SystemTime,
  pub(crate) created: SystemTime,
  pub(crate) modified: SystemTime,
}

impl Metadata for MemoryMetadata {
  fn accessed(&self) -> io::Result<SystemTime> {
    Ok(self.accessed)
  }

  fn created(&self) -> io::Result<SystemTime> {
    Ok(self.created)
  }

  fn file_type(&self) -> Box<dyn FileType + Send + Sync> {
    Box::new(self.file_type)
  }

  fn is_dir(&self) -> bool {
    self.file_type.is_directory
  }

  fn is_file(&self) -> bool {
    self.file_type.is_file
  }

  fn is_symlink(&self) -> bool {
    self.file_type.is_symlink
  }

  fn modified(&self) -> io::Result<SystemTime> {
    Ok(self.modified)
  }

  fn permissions(&self) -> Permissions {
    self.permissions
  }

  fn len(&self) -> u64 {
    self.len
  }
}
