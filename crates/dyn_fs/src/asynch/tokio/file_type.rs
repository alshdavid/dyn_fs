use std::fs;

use crate::asynch::file_system::AsyncFileType;

pub struct AsyncOsFileType(fs::FileType);

impl From<fs::FileType> for AsyncOsFileType {
  fn from(value: fs::FileType) -> Self {
    Self(value)
  }
}

impl AsyncFileType for AsyncOsFileType {
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
