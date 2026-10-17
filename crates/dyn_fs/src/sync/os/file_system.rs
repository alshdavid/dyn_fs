use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use super::OsMetadata;
use super::OsReadDir;
use crate::file_system::DirBuilder;
use crate::file_system::File;
use crate::file_system::FileSystem;
use crate::file_system::FileTimes;
use crate::file_system::Metadata;
use crate::file_system::OpenOptions;
use crate::file_system::Permissions;
use crate::file_system::ReadDir;

#[derive(Default, Debug, Clone)]
pub struct OsFileSystem;

impl FileSystem for OsFileSystem {
  fn open_with(
    &self,
    path: &Path,
    options: &OpenOptions,
  ) -> io::Result<Box<dyn File + Send + Sync>> {
    let mut open_options = fs::OpenOptions::new();
    open_options
      .read(options.get_read())
      .write(options.get_write())
      .append(options.get_append())
      .truncate(options.get_truncate())
      .create(options.get_create())
      .create_new(options.get_create_new());

    Ok(Box::new(open_options.open(path)?))
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

  fn create_dir_with(
    &self,
    path: &Path,
    builder: &DirBuilder,
  ) -> io::Result<()> {
    let mut dir_builder = fs::DirBuilder::new();
    dir_builder.recursive(builder.get_recursive());
    #[cfg(unix)]
    if let Some(mode) = builder.get_mode() {
      dir_builder.mode(mode);
    }
    dir_builder.create(path)
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

  fn set_times(
    &self,
    path: &Path,
    times: &FileTimes,
  ) -> io::Result<()> {
    let file = fs::File::open(path)?;
    file.set_times(to_std_file_times(times))
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

  fn try_exists(
    &self,
    path: &Path,
  ) -> io::Result<bool> {
    path.try_exists()
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
    fs::File::set_times(self, to_std_file_times(times))
  }

  fn sync_all(&self) -> io::Result<()> {
    fs::File::sync_all(self)
  }

  fn sync_data(&self) -> io::Result<()> {
    fs::File::sync_data(self)
  }
}

fn to_std_file_times(times: &FileTimes) -> fs::FileTimes {
  let mut file_times = fs::FileTimes::new();
  if let Some(accessed) = times.accessed() {
    file_times = file_times.set_accessed(accessed);
  }
  if let Some(modified) = times.modified() {
    file_times = file_times.set_modified(modified);
  }
  file_times
}
