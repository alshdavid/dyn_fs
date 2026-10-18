use std::io;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use super::IncludeDirFileType;
use super::IncludeDirMetadata;
use crate::File;
use crate::FileTimes;
use crate::Metadata;
use crate::Permissions;

/// A read-only file handle onto an embedded `include_dir` file.
///
/// Reads and seeks delegate to an in-memory cursor over the file's embedded
/// bytes. Writes always fail with [`io::ErrorKind::Unsupported`] because the
/// embedded contents are read-only.
pub struct IncludeDirFile {
  pub(crate) cursor: Cursor<&'static [u8]>,
}

fn read_only() -> io::Error {
  io::Error::new(
    io::ErrorKind::Unsupported,
    "include_dir file system is read-only",
  )
}

impl Read for IncludeDirFile {
  fn read(
    &mut self,
    buf: &mut [u8],
  ) -> io::Result<usize> {
    self.cursor.read(buf)
  }
}

impl Seek for IncludeDirFile {
  fn seek(
    &mut self,
    position: SeekFrom,
  ) -> io::Result<u64> {
    self.cursor.seek(position)
  }
}

impl Write for IncludeDirFile {
  fn write(
    &mut self,
    _buf: &[u8],
  ) -> io::Result<usize> {
    Err(read_only())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

impl File for IncludeDirFile {
  fn metadata(&self) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    Ok(Box::new(IncludeDirMetadata {
      file_type: IncludeDirFileType::file(),
      len: self.cursor.get_ref().len() as u64,
    }))
  }

  fn set_len(
    &self,
    _size: u64,
  ) -> io::Result<()> {
    Err(read_only())
  }

  fn set_permissions(
    &self,
    _perm: &Permissions,
  ) -> io::Result<()> {
    Err(read_only())
  }

  fn set_times(
    &self,
    _times: &FileTimes,
  ) -> io::Result<()> {
    Err(read_only())
  }

  fn sync_all(&self) -> io::Result<()> {
    Ok(())
  }

  fn sync_data(&self) -> io::Result<()> {
    Ok(())
  }
}
