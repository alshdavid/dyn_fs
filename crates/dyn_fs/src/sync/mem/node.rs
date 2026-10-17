use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::SystemTime;

use super::MemoryFileType;
use super::MemoryMetadata;
use crate::file_system::Permissions;

/// The in-memory node table, keyed by normalized path.
pub(crate) type Nodes = HashMap<PathBuf, Node>;

/// A single entry within the in-memory file system.
#[derive(Debug, Clone)]
pub(crate) struct Node {
  pub kind: NodeKind,
  pub permissions: Permissions,
  pub accessed: SystemTime,
  pub created: SystemTime,
  pub modified: SystemTime,
}

/// The kind-specific data of a [`Node`].
#[derive(Debug, Clone)]
pub(crate) enum NodeKind {
  /// A file whose contents are shared between hard links.
  File(Arc<RwLock<Vec<u8>>>),
  /// A directory.
  Directory,
  /// A symbolic link pointing at the given (possibly relative) target.
  Symlink(PathBuf),
}

impl Node {
  /// Creates a new file node with the given contents.
  pub fn file(contents: Vec<u8>) -> Self {
    let now = SystemTime::now();
    Self {
      kind: NodeKind::File(Arc::new(RwLock::new(contents))),
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
      kind: NodeKind::Directory,
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
      kind: NodeKind::Symlink(target),
      permissions: Permissions::new(),
      accessed: now,
      created: now,
      modified: now,
    }
  }

  /// Returns `true` if this node is a directory.
  pub fn is_dir(&self) -> bool {
    matches!(&self.kind, NodeKind::Directory)
  }

  /// Returns the file type of this node.
  pub fn file_type(&self) -> MemoryFileType {
    match &self.kind {
      NodeKind::File(_) => MemoryFileType::file(),
      NodeKind::Directory => MemoryFileType::directory(),
      NodeKind::Symlink(_) => MemoryFileType::symlink(),
    }
  }

  /// Returns the size of this node in bytes.
  pub fn len(&self) -> u64 {
    match &self.kind {
      NodeKind::File(data) => data.read().map(|data| data.len() as u64).unwrap_or(0),
      NodeKind::Symlink(target) => target.as_os_str().len() as u64,
      NodeKind::Directory => 0,
    }
  }

  /// Produces a metadata snapshot of this node.
  pub fn metadata(&self) -> MemoryMetadata {
    MemoryMetadata {
      file_type: self.file_type(),
      len: self.len(),
      permissions: self.permissions,
      accessed: self.accessed,
      created: self.created,
      modified: self.modified,
    }
  }
}
