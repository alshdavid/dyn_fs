use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;

use super::AsyncMemoryFileType;
use super::AsyncMemoryMetadata;
use crate::asynch::file_system::Permissions;

/// The in-memory node table, keyed by normalized path.
pub(crate) type AsyncNodes = HashMap<PathBuf, AsyncNode>;

/// A single entry within the in-memory file system.
#[derive(Debug, Clone)]
pub(crate) struct AsyncNode {
  pub kind: AsyncNodeKind,
  pub permissions: Permissions,
  pub accessed: SystemTime,
  pub created: SystemTime,
  pub modified: SystemTime,
}

/// The kind-specific data of a [`AsyncNode`].
#[derive(Debug, Clone)]
pub(crate) enum AsyncNodeKind {
  /// A file whose contents are shared between hard links.
  File(Arc<RwLock<Vec<u8>>>),
  /// A directory.
  Directory,
  /// A symbolic link pointing at the given (possibly relative) target.
  Symlink(PathBuf),
}

impl AsyncNode {
  /// Creates a new file node with the given contents.
  pub fn file(contents: Vec<u8>) -> Self {
    let now = SystemTime::now();
    Self {
      kind: AsyncNodeKind::File(Arc::new(RwLock::new(contents))),
      permissions: Permissions::new(),
      accessed: now,
      created: now,
      modified: now,
    }
  }

  /// Creates a new directory node.
  pub fn directory() -> Self {
    let now = SystemTime::now();
    Self {
      kind: AsyncNodeKind::Directory,
      permissions: Permissions::new(),
      accessed: now,
      created: now,
      modified: now,
    }
  }

  /// Creates a new symbolic link node pointing at `target`.
  pub fn symlink(target: PathBuf) -> Self {
    let now = SystemTime::now();
    Self {
      kind: AsyncNodeKind::Symlink(target),
      permissions: Permissions::new(),
      accessed: now,
      created: now,
      modified: now,
    }
  }

  /// Returns `true` if this node is a directory.
  pub fn is_dir(&self) -> bool {
    matches!(&self.kind, AsyncNodeKind::Directory)
  }

  /// Returns the file type of this node.
  pub fn file_type(&self) -> AsyncMemoryFileType {
    match &self.kind {
      AsyncNodeKind::File(_) => AsyncMemoryFileType::file(),
      AsyncNodeKind::Directory => AsyncMemoryFileType::directory(),
      AsyncNodeKind::Symlink(_) => AsyncMemoryFileType::symlink(),
    }
  }

  /// Returns the size of this node in bytes.
  pub fn len(&self) -> u64 {
    match &self.kind {
      AsyncNodeKind::File(data) => data.read().map_or(0, |data| data.len() as u64),
      AsyncNodeKind::Symlink(target) => target.as_os_str().len() as u64,
      AsyncNodeKind::Directory => 0,
    }
  }

  /// Produces a metadata snapshot of this node.
  pub fn metadata(&self) -> AsyncMemoryMetadata {
    AsyncMemoryMetadata {
      file_type: self.file_type(),
      len: self.len(),
      permissions: self.permissions,
      accessed: self.accessed,
      created: self.created,
      modified: self.modified,
    }
  }
}
