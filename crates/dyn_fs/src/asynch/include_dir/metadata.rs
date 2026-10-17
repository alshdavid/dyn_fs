use std::io;
use std::time::SystemTime;

use super::AsyncIncludeDirFileType;
use crate::asynch::file_system::AsyncFileType;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::Permissions;

/// Metadata for an embedded `include_dir` entry.
///
/// `include_dir` does not record timestamps by default, so the timestamp
/// accessors report [`SystemTime::UNIX_EPOCH`]. Embedded entries are always
/// read-only.
#[derive(Debug, Clone, Copy)]
pub struct AsyncIncludeDirMetadata {
  pub(crate) file_type: AsyncIncludeDirFileType,
  pub(crate) len: u64,
}

impl AsyncMetadata for AsyncIncludeDirMetadata {
  fn accessed(&self) -> io::Result<SystemTime> {
    Ok(SystemTime::UNIX_EPOCH)
  }

  fn created(&self) -> io::Result<SystemTime> {
    Ok(SystemTime::UNIX_EPOCH)
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
    false
  }

  fn modified(&self) -> io::Result<SystemTime> {
    Ok(SystemTime::UNIX_EPOCH)
  }

  fn permissions(&self) -> Permissions {
    Permissions::from_readonly(true)
  }

  fn len(&self) -> u64 {
    self.len
  }
}
