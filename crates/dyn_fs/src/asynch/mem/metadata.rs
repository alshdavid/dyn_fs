use std::io;
use std::time::SystemTime;

use super::AsyncMemoryFileType;
use crate::asynch::file_system::AsyncFileType;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::AsyncPermissions;

/// A metadata snapshot of an in-memory file system node.
#[derive(Debug, Clone)]
pub struct AsyncMemoryMetadata {
  pub(crate) file_type: AsyncMemoryFileType,
  pub(crate) len: u64,
  pub(crate) permissions: AsyncPermissions,
  pub(crate) accessed: SystemTime,
  pub(crate) created: SystemTime,
  pub(crate) modified: SystemTime,
}

impl AsyncMetadata for AsyncMemoryMetadata {
  fn accessed(&self) -> io::Result<SystemTime> {
    Ok(self.accessed)
  }

  fn created(&self) -> io::Result<SystemTime> {
    Ok(self.created)
  }

  fn file_type(&self) -> Box<dyn AsyncFileType + Send + Sync> {
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

  fn permissions(&self) -> AsyncPermissions {
    self.permissions
  }

  fn len(&self) -> u64 {
    self.len
  }
}
