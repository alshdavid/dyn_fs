use crate::file_system::Metadata;

pub struct MemoryMetadata {
  pub(super) inner_is_file: bool,
  pub(super) inner_is_dir: bool,
}

impl Metadata for MemoryMetadata {
  fn is_dir(&self) -> bool {
    self.inner_is_dir
  }

  fn is_file(&self) -> bool {
    self.inner_is_file
  }

  fn accessed(&self) -> std::io::Result<std::time::SystemTime> {
    todo!()
  }

  fn created(&self) -> std::io::Result<std::time::SystemTime> {
    todo!()
  }

  fn file_type(&self) -> Box<dyn crate::file_system::FileType + Send + Sync> {
    todo!()
  }

  fn is_symlink(&self) -> bool {
    todo!()
  }

  fn modified(&self) -> std::io::Result<std::time::SystemTime> {
    todo!()
  }

  fn permissions(&self) -> Box<dyn crate::file_system::Permissions + Send + Sync> {
    todo!()
  }

  fn len(&self) -> u64 {
    todo!()
  }
}
