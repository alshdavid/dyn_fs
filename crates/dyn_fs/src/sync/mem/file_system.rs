use std::collections::HashMap;
use std::io;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

use normalize_path::NormalizePath;

use super::MemoryMetadata;
use crate::file_system::File;
use crate::file_system::FileSystem;
use crate::file_system::FileTimes;
use crate::file_system::Metadata;
use crate::file_system::Permissions;
use crate::file_system::ReadDir;

#[derive(Debug)]
enum Entry {
  File(Vec<u8>),
  Directory,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryFileSystem {
  files: Arc<RwLock<HashMap<PathBuf, Entry>>>,
}

impl MemoryFileSystem {
  fn files_write<'a>(&'a self) -> io::Result<RwLockWriteGuard<'a, HashMap<PathBuf, Entry>>> {
    let Ok(files) = self.files.write() else {
      return Err(io::Error::other("Unable to unlock mutex"));
    };
    Ok(files)
  }

  fn files_read<'a>(&'a self) -> io::Result<RwLockReadGuard<'a, HashMap<PathBuf, Entry>>> {
    let Ok(files) = self.files.read() else {
      return Err(io::Error::other("Unable to unlock mutex"));
    };
    Ok(files)
  }
}

impl FileSystem for MemoryFileSystem {
  fn open(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn File>> {
    Ok(Box::new(Cursor::new(self.read(path)?)))
  }

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>> {
    let path = path.normalize();

    let files = self.files_read()?;

    let Some(file) = files.get(&path) else {
      return Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found",
      ));
    };

    match file {
      Entry::File(contents) => Ok(contents.clone()),
      Entry::Directory => Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Path is a directory",
      )),
    }
  }

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String> {
    let path = path.normalize();
    let bytes = self.read(&path)?;
    String::from_utf8(bytes).map_err(|_| std::io::Error::other("Unable to read file as string"))
  }

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata>> {
    let path = path.normalize();
    let files = self.files_read()?;
    let file = files.get(&path);

    Ok(Box::new(MemoryMetadata {
      inner_is_dir: matches!(file, Some(Entry::Directory { .. })),
      inner_is_file: matches!(file, Some(Entry::File { .. })),
    }))
  }

  fn write(
    &self,
    path: &Path,
    contents: &[u8],
  ) -> io::Result<()> {
    let path = path.normalize();

    let mut files = self.files_write()?;
    files.insert(path.clone(), Entry::File(contents.to_vec()));

    let mut dir = path.parent();
    while let Some(path) = dir {
      files.insert(path.to_path_buf(), Entry::Directory);
      dir = path.parent();
    }

    Ok(())
  }

  fn canonicalize(
    &self,
    _path: &Path,
  ) -> io::Result<PathBuf> {
    todo!()
  }

  fn read_link(
    &self,
    _path: &Path,
  ) -> io::Result<PathBuf> {
    todo!()
  }

  fn copy(
    &self,
    _from: &Path,
    _to: &Path,
  ) -> io::Result<u64> {
    todo!()
  }

  fn create_dir(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn create_dir_all(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn hard_link(
    &self,
    _original: &Path,
    _link: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn read_dir(
    &self,
    _path: &Path,
  ) -> io::Result<Box<dyn ReadDir>> {
    todo!()
  }

  fn remove_dir(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn remove_dir_all(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn remove_file(
    &self,
    _path: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn rename(
    &self,
    _from: &Path,
    _to: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn set_permissions(
    &self,
    _path: &Path,
    _perm: &dyn crate::file_system::Permissions,
  ) -> io::Result<()> {
    todo!()
  }

  fn symlink(
    &self,
    _original: &Path,
    _link: &Path,
  ) -> io::Result<()> {
    todo!()
  }

  fn symlink_metadata(
    &self,
    _path: &Path,
  ) -> io::Result<Box<dyn Metadata>> {
    todo!()
  }
}

impl File for Cursor<Vec<u8>> {
  fn metadata(&self) -> io::Result<Box<dyn Metadata>> {
    Ok(Box::new(MemoryMetadata {
      inner_is_file: true,
      inner_is_dir: false,
    }))
  }

  fn set_len(
    &self,
    _size: u64,
  ) -> io::Result<()> {
    todo!()
  }

  fn set_permissions(
    &self,
    _perm: &dyn Permissions,
  ) -> io::Result<()> {
    todo!()
  }

  fn set_times(
    &self,
    _times: &FileTimes,
  ) -> io::Result<()> {
    todo!()
  }

  fn sync_all(&self) -> io::Result<()> {
    Ok(())
  }

  fn sync_data(&self) -> io::Result<()> {
    Ok(())
  }
}
