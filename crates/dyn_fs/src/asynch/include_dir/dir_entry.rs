use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use super::AsyncIncludeDirMetadata;
use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncFileType;
use crate::asynch::file_system::AsyncMetadata;

/// A directory entry of the embedded `include_dir` file system.
#[derive(Debug, Clone)]
pub struct AsyncIncludeDirDirEntry {
  pub(crate) path: PathBuf,
  pub(crate) metadata: AsyncIncludeDirMetadata,
}

impl AsyncDirEntry for AsyncIncludeDirDirEntry {
  fn path(&self) -> PathBuf {
    self.path.clone()
  }

  fn file_name(&self) -> OsString {
    self
      .path
      .file_name()
      .map(|name| name.to_os_string())
      .unwrap_or_default()
  }

  fn file_type(&self) -> io::Result<Box<dyn AsyncFileType + Send + Sync>> {
    Ok(Box::new(self.metadata.file_type))
  }

  fn metadata(&self) -> io::Result<Box<dyn AsyncMetadata + Send + Sync>> {
    Ok(Box::new(self.metadata))
  }
}
