use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use super::OsMetadata;
use super::OsReadDir;
use crate::file_system::File;
use crate::file_system::FileSystem;
use crate::file_system::FileTimes;
use crate::file_system::Metadata;
use crate::file_system::Permissions;
use crate::file_system::ReadDir;

#[derive(Default, Debug, Clone)]
pub struct OsFileSystem;

impl FileSystem for OsFileSystem {
  fn open(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn File + Send + Sync>> {
    Ok(Box::new(fs::File::open(path)?))
  }

  fn canonicalize(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf> {
    fs::canonicalize(path)
  }

  fn read_link(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf> {
    fs::read_link(path)
  }

  fn copy(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<u64> {
    fs::copy(from, to)
  }

  fn create_dir(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    fs::create_dir(path)
  }

  fn create_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    fs::create_dir_all(path)
  }

  fn hard_link(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()> {
    fs::hard_link(original, link)
  }

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let metadata = fs::metadata(path)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>> {
    fs::read(path)
  }

  fn read_dir(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn ReadDir + Send + Sync>> {
    let read_dir = fs::read_dir(path)?;
    Ok(Box::new(OsReadDir::from(read_dir)))
  }

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String> {
    fs::read_to_string(path)
  }

  fn remove_dir(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    fs::remove_dir(path)
  }

  fn remove_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    fs::remove_dir_all(path)
  }

  fn remove_file(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    fs::remove_file(path)
  }

  fn rename(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<()> {
    fs::rename(from, to)
  }

  #[cfg(not(unix))]
  fn set_permissions(
    &self,
    path: &Path,
    perm: &Permissions,
  ) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(perm.readonly());
    fs::set_permissions(path, permissions)
  }

  #[cfg(unix)]
  fn set_permissions(
    &self,
    path: &Path,
    perm: &Permissions,
  ) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(perm.mode()))
  }

  #[cfg(unix)]
  fn symlink(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()> {
    std::os::unix::fs::symlink(original, link)
  }

  #[cfg(windows)]
  fn symlink(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()> {
    if fs::metadata(original).map(|m| m.is_dir()).unwrap_or(false) {
      std::os::windows::fs::symlink_dir(original, link)
    } else {
      std::os::windows::fs::symlink_file(original, link)
    }
  }

  fn symlink_metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn write(
    &self,
    path: &Path,
    contents: &[u8],
  ) -> io::Result<()> {
    fs::write(path, contents)
  }
}

impl File for fs::File {
  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let metadata = fs::File::metadata(self)?;
    Ok(Box::new(OsMetadata::from(metadata)))
  }

  fn set_len(
    &self,
    size: u64,
  ) -> io::Result<()> {
    fs::File::set_len(self, size)
  }

  #[cfg(unix)]
  fn set_permissions(
    &self,
    perm: &Permissions,
  ) -> io::Result<()> {
    fs::File::set_permissions(self, fs::Permissions::from_mode(perm.mode()))
  }

  #[cfg(not(unix))]
  fn set_permissions(
    &self,
    perm: &Permissions,
  ) -> io::Result<()> {
    let mut permissions = fs::File::metadata(self)?.permissions();
    permissions.set_readonly(perm.readonly());
    fs::File::set_permissions(self, permissions)
  }

  fn set_times(
    &self,
    times: &FileTimes,
  ) -> io::Result<()> {
    let mut file_times = fs::FileTimes::new();
    if let Some(accessed) = times.accessed() {
      file_times = file_times.set_accessed(accessed);
    }
    if let Some(modified) = times.modified() {
      file_times = file_times.set_modified(modified);
    }
    fs::File::set_times(self, file_times)
  }

  fn sync_all(&self) -> io::Result<()> {
    fs::File::sync_all(self)
  }

  fn sync_data(&self) -> io::Result<()> {
    fs::File::sync_data(self)
  }
}
