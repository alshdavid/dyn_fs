use std::path::Path;
use std::path::PathBuf;

use super::os_string_ext::OsStringExt;

pub trait PathExt {
  fn try_parent(&self) -> std::io::Result<&Path>;
  fn try_file_name(&self) -> std::io::Result<String>;
  fn try_extension(&self) -> std::io::Result<String>;
  fn try_file_stem(&self) -> std::io::Result<String>;
  fn try_to_string(&self) -> std::io::Result<String>;
}

impl PathExt for PathBuf {
  fn try_parent(&self) -> std::io::Result<&Path> {
    match self.parent() {
      Some(path) => Ok(path),
      None => Err(std::io::Error::other("Unable to find parent")),
    }
  }

  fn try_file_name(&self) -> std::io::Result<String> {
    match self.file_name() {
      Some(v) => Ok(v.try_to_string()?),
      None => Err(std::io::Error::other("Cannot get file name")),
    }
  }

  fn try_file_stem(&self) -> std::io::Result<String> {
    match self.file_stem() {
      Some(v) => Ok(v.try_to_string()?),
      None => Err(std::io::Error::other("Cannot get file stem")),
    }
  }

  fn try_to_string(&self) -> std::io::Result<String> {
    match self.to_str() {
      Some(v) => Ok(v.to_string()),
      None => Err(std::io::Error::other("Cannot convert Path to string")),
    }
  }

  fn try_extension(&self) -> std::io::Result<String> {
    match self.extension() {
      Some(ext) => ext.try_to_string(),
      None => Err(std::io::Error::other("Cannot get extension")),
    }
  }
}

impl PathExt for Path {
  fn try_parent(&self) -> std::io::Result<&Path> {
    match self.parent() {
      Some(path) => Ok(path),
      None => Err(std::io::Error::other("Unable to find parent")),
    }
  }

  fn try_file_name(&self) -> std::io::Result<String> {
    match self.file_name() {
      Some(v) => Ok(v.try_to_string()?),
      None => Err(std::io::Error::other("Cannot get file name")),
    }
  }

  fn try_file_stem(&self) -> std::io::Result<String> {
    match self.file_stem() {
      Some(v) => Ok(v.try_to_string()?),
      None => Err(std::io::Error::other("Cannot get file stem")),
    }
  }

  fn try_to_string(&self) -> std::io::Result<String> {
    match self.to_str() {
      Some(v) => Ok(v.to_string()),
      None => Err(std::io::Error::other("Cannot convert Path to string")),
    }
  }

  fn try_extension(&self) -> std::io::Result<String> {
    match self.extension() {
      Some(ext) => ext.try_to_string(),
      None => Err(std::io::Error::other("Cannot get extension")),
    }
  }
}
