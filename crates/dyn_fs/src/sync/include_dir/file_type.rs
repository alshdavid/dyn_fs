use crate::FileType;

/// File type of an embedded `include_dir` entry.
///
/// Embedded assets never contain symbolic links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncludeDirFileType {
  pub(crate) is_directory: bool,
  pub(crate) is_file: bool,
}

impl IncludeDirFileType {
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

impl FileType for IncludeDirFileType {
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
