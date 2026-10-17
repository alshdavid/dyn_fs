use crate::file_system::FileType;

/// File type of an in-memory node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryFileType {
  pub(crate) is_directory: bool,
  pub(crate) is_file: bool,
  pub(crate) is_symlink: bool,
}

impl MemoryFileType {
  pub(crate) fn file() -> Self {
    Self {
      is_directory: false,
      is_file: true,
      is_symlink: false,
    }
  }

  pub(crate) fn directory() -> Self {
    Self {
      is_directory: true,
      is_file: false,
      is_symlink: false,
    }
  }

  pub(crate) fn symlink() -> Self {
    Self {
      is_directory: false,
      is_file: false,
      is_symlink: true,
    }
  }
}

impl FileType for MemoryFileType {
  fn is_dir(&self) -> bool {
    self.is_directory
  }

  fn is_file(&self) -> bool {
    self.is_file
  }

  fn is_symlink(&self) -> bool {
    self.is_symlink
  }
}
