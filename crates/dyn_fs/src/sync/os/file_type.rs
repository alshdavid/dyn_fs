use std::fs;

use crate::FileType;

pub struct OsFileType(fs::FileType);

impl From<fs::FileType> for OsFileType {
  fn from(value: fs::FileType) -> Self {
    Self(value)
  }
}

impl FileType for OsFileType {
  fn is_dir(&self) -> bool {
    self.0.is_dir()
  }

  fn is_file(&self) -> bool {
    self.0.is_file()
  }

  fn is_symlink(&self) -> bool {
    self.0.is_symlink()
  }
}
