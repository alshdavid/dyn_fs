//! Abstraction of the file system
//!
//! This module contains traits that map the functionality of
//! the Rust standard library FileSystem to trait methods

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

pub trait FileSystem: Send + Sync {
  fn open(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn File>>;

  fn canonicalize(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf>;

  fn read_link(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf>;

  fn copy(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<u64>;

  fn create_dir(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn create_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn hard_link(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()>;

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata>>;

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>>;

  fn read_dir(
    &self,
    path: &Path,
  ) -> io::Result<fs::ReadDir>;

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String>;

  fn remove_dir(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn remove_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn remove_file(
    &self,
    path: &Path,
  ) -> io::Result<()>;

  fn rename(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<()>;

  fn set_permissions(
    &self,
    path: &Path,
    perm: &dyn Permissions,
  ) -> io::Result<()>;

  fn symlink_metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata>>;

  fn write(
    &self,
    path: &Path,
    contents: &[u8],
  ) -> io::Result<()>;
}

pub trait Metadata {
  fn accessed(&self) -> io::Result<SystemTime>;
  fn created(&self) -> io::Result<SystemTime>;
  fn file_type(&self) -> Box<dyn FileType>;
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
  fn modified(&self) -> io::Result<SystemTime>;
  fn permissions(&self) -> Box<dyn Permissions>;
  fn len(&self) -> u64;
}

pub trait Permissions {
  fn readonly(&self) -> bool;

  fn set_readonly(
    &mut self,
    readonly: bool,
  );

  #[cfg(unix)]
  fn mode(&self) -> u32;

  #[cfg(unix)]
  fn set_mode(
    &mut self,
    mode: u32,
  );
}

pub trait FileType {
  fn is_dir(&self) -> bool;
  fn is_file(&self) -> bool;
  fn is_symlink(&self) -> bool;
}

pub trait File: io::Read + io::Write {}
