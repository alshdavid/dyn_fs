use std::fs;
use std::time::SystemTime;

use super::OsFileType;
use crate::file_system::FileType;
use crate::file_system::Metadata;
use crate::file_system::Permissions;

pub struct OsMetadata(fs::Metadata);

impl From<fs::Metadata> for OsMetadata {
  fn from(value: fs::Metadata) -> Self {
    Self(value)
  }
}

impl Metadata for OsMetadata {
  fn accessed(&self) -> std::io::Result<SystemTime> {
    self.0.accessed()
  }

  fn created(&self) -> std::io::Result<SystemTime> {
    self.0.created()
  }

  fn file_type(&self) -> Box<dyn FileType + Send + Sync> {
    let file_type = self.0.file_type();
    Box::new(OsFileType::from(file_type))
  }

  fn is_dir(&self) -> bool {
    self.0.is_dir()
  }

  fn is_file(&self) -> bool {
    self.0.is_file()
  }

  fn is_symlink(&self) -> bool {
    self.0.is_symlink()
  }

  fn modified(&self) -> std::io::Result<SystemTime> {
    self.0.modified()
  }

  fn permissions(&self) -> Permissions {
    let permissions = self.0.permissions();
    Permissions::from(permissions)
  }

  fn len(&self) -> u64 {
    self.0.len()
  }
}
