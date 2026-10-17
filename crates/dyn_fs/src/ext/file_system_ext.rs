use std::path::Path;
use std::path::PathBuf;

use crate::sync::FileSystem;

pub trait FileSystemExt {
  fn find_ancestor_file<P: AsRef<Path>, S: AsRef<Path>>(
    &self,
    start_dir: P,
    file_name: S,
  ) -> std::io::Result<Vec<PathBuf>>;
}

impl<F: FileSystem> FileSystemExt for F {
  fn find_ancestor_file<P: AsRef<Path>, S: AsRef<Path>>(
    &self,
    start_dir: P,
    file_name: S,
  ) -> std::io::Result<Vec<PathBuf>> {
    let file_name = file_name.as_ref();
    let mut found = vec![];
    let mut current = start_dir.as_ref().to_path_buf();

    loop {
      let possible = current.join(file_name);

      if self.try_exists(&possible)? {
        found.push(possible)
      }

      let Some(next) = current.parent() else {
        break;
      };

      current = next.to_path_buf();
    }

    Ok(found)
  }
}
