use crate::asynch::file_system::AsyncFileType;

/// File type of an embedded `include_dir` entry.
///
/// Embedded assets never contain symbolic links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsyncIncludeDirFileType {
  pub(crate) is_directory: bool,
  pub(crate) is_file: bool,
}

impl AsyncIncludeDirFileType {
  pub(crate) fn directory() -> Self {
    Self {
      is_directory: true,
      is_file: false,
    }
  }

  pub(crate) fn file() -> Self {
    Self {
      is_directory: false,
      is_file: true,
    }
  }
}

impl AsyncFileType for AsyncIncludeDirFileType {
  fn is_dir(&self) -> bool {
    self.is_directory
  }

  fn is_file(&self) -> bool {
    self.is_file
  }

  fn is_symlink(&self) -> bool {
    false
  }
}
