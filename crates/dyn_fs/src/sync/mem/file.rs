use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;

use super::node::Node;
use super::node::NodeKind;
use crate::File;
use crate::FileTimes;
use crate::Metadata;
use crate::Permissions;

/// A handle onto a file within a [`super::MemoryFileSystem`].
///
/// Reads and writes operate directly on the shared node table, so changes are
/// visible to the file system immediately, and to any hard links of the file.
pub struct MemoryFile {
  pub(crate) files: Arc<RwLock<HashMap<PathBuf, Node>>>,
  pub(crate) path: PathBuf,
  pub(crate) position: u64,
  pub(crate) append: bool,
}

impl MemoryFile {
  fn poisoned() -> io::Error {
    io::Error::other("memory file system lock poisoned")
  }

  /// Returns the shared, mutable contents of the underlying file.
  fn data(&self) -> io::Result<Arc<RwLock<Vec<u8>>>> {
    let files = self.files.read().map_err(|_| Self::poisoned())?;
    match files.get(&self.path) {
      Some(Node {
        kind: NodeKind::File(data),
        ..
      }) => Ok(Arc::clone(data)),
      Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
      None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
    }
  }

  /// Updates the modified timestamp of the underlying file.
  fn touch(&self) -> io::Result<()> {
    let mut files = self.files.write().map_err(|_| Self::poisoned())?;
    if let Some(node) = files.get_mut(&self.path) {
      node.modified = SystemTime::now();
    }
    Ok(())
  }
}

impl Read for MemoryFile {
  fn read(
    &mut self,
    buf: &mut [u8],
  ) -> io::Result<usize> {
    let data = self.data()?;
    let data = data.read().map_err(|_| Self::poisoned())?;

    let start = self.position as usize;
    if start >= data.len() {
      return Ok(0);
    }

    let end = std::cmp::min(start + buf.len(), data.len());
    let length = end - start;

    buf[..length].copy_from_slice(&data[start..end]);
    self.position += length as u64;

    Ok(length)
  }
}

impl Write for MemoryFile {
  fn write(
    &mut self,
    buf: &[u8],
  ) -> io::Result<usize> {
    let data = self.data()?;
    {
      let mut data = data.write().map_err(|_| Self::poisoned())?;

      if self.append {
        self.position = data.len() as u64;
      }

      let start = self.position as usize;
      if start > data.len() {
        data.resize(start, 0);
      }

      let end = start + buf.len();
      if end > data.len() {
        data.resize(end, 0);
      }

      data[start..end].copy_from_slice(buf);
      self.position = end as u64;
    }

    self.touch()?;

    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

impl Seek for MemoryFile {
  fn seek(
    &mut self,
    position: SeekFrom,
  ) -> io::Result<u64> {
    let length = {
      let data = self.data()?;
      let data = data.read().map_err(|_| Self::poisoned())?;
      data.len() as u64
    };

    let next = match position {
      SeekFrom::Start(offset) => offset as i64,
      SeekFrom::Current(offset) => self.position as i64 + offset,
      SeekFrom::End(offset) => length as i64 + offset,
    };

    if next < 0 {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid seek to a negative position",
      ));
    }

    self.position = next as u64;

    Ok(self.position)
  }
}

impl File for MemoryFile {
  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let files = self.files.read().map_err(|_| Self::poisoned())?;
    let node = files
      .get(&self.path)
      .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))?;
    Ok(Box::new(node.metadata()))
  }

  fn set_len(
    &self,
    size: u64,
  ) -> io::Result<()> {
    let data = self.data()?;
    data
      .write()
      .map_err(|_| Self::poisoned())?
      .resize(size as usize, 0);
    self.touch()?;
    Ok(())
  }

  fn set_permissions(
    &self,
    perm: &Permissions,
  ) -> io::Result<()> {
    let mut files = self.files.write().map_err(|_| Self::poisoned())?;
    match files.get_mut(&self.path) {
      Some(node) => {
        node.permissions = *perm;
        Ok(())
      }
      None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
    }
  }

  fn set_times(
    &self,
    times: &FileTimes,
  ) -> io::Result<()> {
    let mut files = self.files.write().map_err(|_| Self::poisoned())?;
    match files.get_mut(&self.path) {
      Some(node) => {
        if let Some(accessed) = times.accessed() {
          node.accessed = accessed;
        }
        if let Some(modified) = times.modified() {
          node.modified = modified;
        }
        Ok(())
      }
      None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
    }
  }

  fn sync_all(&self) -> io::Result<()> {
    Ok(())
  }

  fn sync_data(&self) -> io::Result<()> {
    Ok(())
  }
}
