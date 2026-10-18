use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

use normalize_path::NormalizePath;

use super::MemoryDirEntry;
use super::MemoryFile;
use super::MemoryMetadata;
use super::node::Node;
use super::node::NodeKind;
use super::node::Nodes;
use crate::DirBuilder;
use crate::DirEntry;
use crate::File;
use crate::FileSystem;
use crate::FileTimes;
use crate::Metadata;
use crate::OpenOptions;
use crate::Permissions;
use crate::ReadDir;

/// An in-memory, thread-safe implementation of [`FileSystem`].
///
/// The file system is a flat table of normalized paths mapping to nodes. Every
/// mutating operation routes through the shared table, so clones of a
/// `MemoryFileSystem` observe the same data.
#[derive(Default, Debug, Clone)]
pub struct MemoryFileSystem {
  files: Arc<RwLock<Nodes>>,
}

impl MemoryFileSystem {
  /// Creates an empty in-memory file system, returned as a shared [`Arc`] trait
  /// object.
  #[allow(clippy::new_ret_no_self)]
  #[must_use]
  pub fn new() -> Arc<dyn FileSystem> {
    Arc::new(Self::default())
  }

  fn files_write(&self) -> io::Result<RwLockWriteGuard<'_, Nodes>> {
    let Ok(files) = self.files.write() else {
      return Err(io::Error::other("memory file system lock poisoned"));
    };
    Ok(files)
  }

  fn files_read(&self) -> io::Result<RwLockReadGuard<'_, Nodes>> {
    let Ok(files) = self.files.read() else {
      return Err(io::Error::other("memory file system lock poisoned"));
    };
    Ok(files)
  }
}

fn not_found() -> io::Error {
  io::Error::new(io::ErrorKind::NotFound, "path not found")
}

fn already_exists() -> io::Error {
  io::Error::new(io::ErrorKind::AlreadyExists, "path already exists")
}

fn not_a_directory() -> io::Error {
  io::Error::new(io::ErrorKind::NotADirectory, "not a directory")
}

fn is_a_directory() -> io::Error {
  io::Error::new(io::ErrorKind::IsADirectory, "is a directory")
}

fn poisoned() -> io::Error {
  io::Error::other("memory file system lock poisoned")
}

/// Ensures every ancestor directory of `path` exists.
fn materialize_parents(
  nodes: &mut Nodes,
  path: &Path,
) {
  let mut parent = path.parent();
  while let Some(directory) = parent {
    nodes
      .entry(directory.to_path_buf())
      .or_insert_with(Node::directory);

    if directory.as_os_str().is_empty() {
      break;
    }

    parent = directory.parent();
  }
}

/// Returns an error unless the parent of `path` exists and is a directory.
fn parent_is_directory(
  nodes: &Nodes,
  path: &Path,
) -> io::Result<()> {
  match path.parent() {
    None => Ok(()),
    Some(parent) if parent.as_os_str().is_empty() => Ok(()),
    Some(parent) => match nodes.get(parent) {
      Some(node) if node.is_dir() => Ok(()),
      Some(_) => Err(not_a_directory()),
      None if parent == Path::new("/") => Ok(()),
      None => Err(not_found()),
    },
  }
}

/// Resolves any symlinks within `path` against the current node table.
fn resolve(
  nodes: &Nodes,
  path: &Path,
) -> io::Result<PathBuf> {
  let mut path = path.normalize();
  let mut budget = 40;

  loop {
    let mut current = PathBuf::new();
    let mut link = None;

    for component in path.components() {
      current.push(component);

      if let Some(Node {
        kind: NodeKind::Symlink(target),
        ..
      }) = nodes.get(&current)
      {
        link = Some((current.clone(), target.clone()));
        break;
      }
    }

    let Some((link_path, target)) = link else {
      return Ok(path);
    };

    if budget == 0 {
      return Err(io::Error::other("too many levels of symbolic links"));
    }
    budget -= 1;

    let next = if target.is_absolute() {
      target
    } else {
      match link_path.parent() {
        Some(parent) => parent.join(&target),
        None => target,
      }
    };

    path = next.normalize();
  }
}

/// Creates one or more directories at `path`.
fn create_directories(
  nodes: &mut Nodes,
  path: &Path,
  recursive: bool,
) -> io::Result<()> {
  if let Some(node) = nodes.get(path) {
    if node.is_dir() {
      return if recursive {
        Ok(())
      } else {
        Err(already_exists())
      };
    }
    return Err(already_exists());
  }

  if !recursive {
    parent_is_directory(nodes, path)?;
    nodes.insert(path.to_path_buf(), Node::directory());
    return Ok(());
  }

  let mut chain = Vec::new();
  let mut current = Some(path);

  while let Some(directory) = current {
    match nodes.get(directory) {
      Some(node) if node.is_dir() => break,
      Some(_) => return Err(not_a_directory()),
      None => {}
    }

    if directory.as_os_str().is_empty() {
      break;
    }

    chain.push(directory.to_path_buf());
    current = directory.parent();
  }

  for directory in chain.into_iter().rev() {
    nodes.insert(directory, Node::directory());
  }

  Ok(())
}

impl FileSystem for MemoryFileSystem {
  fn open_with(
    &self,
    path: &Path,
    options: &OpenOptions,
  ) -> io::Result<Box<dyn File + Send + Sync>> {
    if !options.get_read() && !options.get_write() && !options.get_append() {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "must be opened with read, write, or append access",
      ));
    }

    let path = path.normalize();
    let mut nodes = self.files_write()?;
    let resolved = resolve(&nodes, &path)?;

    let existing = nodes.get(&resolved).map(|node| {
      let is_dir = node.is_dir();
      let data = match &node.kind {
        NodeKind::File(data) => Some(Arc::clone(data)),
        _ => None,
      };
      (is_dir, data)
    });

    match existing {
      Some((true, _)) => return Err(is_a_directory()),
      Some((false, data)) => {
        if options.get_create_new() {
          return Err(already_exists());
        }
        if options.get_truncate()
          && let Some(data) = data
        {
          data.write().map_err(|_| poisoned())?.clear();
        }
      }
      None => {
        if options.get_create() || options.get_create_new() {
          nodes.insert(resolved.clone(), Node::file(Vec::new()));
          materialize_parents(&mut nodes, &resolved);
        } else {
          return Err(not_found());
        }
      }
    }

    drop(nodes);

    Ok(Box::new(MemoryFile {
      files: Arc::clone(&self.files),
      path: resolved,
      position: 0,
      append: options.get_append(),
    }))
  }

  fn read(
    &self,
    path: &Path,
  ) -> io::Result<Vec<u8>> {
    let path = path.normalize();
    let nodes = self.files_read()?;
    let resolved = resolve(&nodes, &path)?;

    match nodes.get(&resolved) {
      Some(Node {
        kind: NodeKind::File(data),
        ..
      }) => {
        let data = data.read().map_err(|_| poisoned())?;
        Ok(data.clone())
      }
      Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
      None => Err(not_found()),
    }
  }

  fn read_to_string(
    &self,
    path: &Path,
  ) -> io::Result<String> {
    let bytes = self.read(path)?;
    String::from_utf8(bytes).map_err(|_| io::Error::other("unable to read file as a UTF-8 string"))
  }

  fn metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let path = path.normalize();
    let nodes = self.files_read()?;
    let resolved = resolve(&nodes, &path)?;

    match nodes.get(&resolved) {
      Some(node) => Ok(Box::new(node.metadata())),
      None => Err(not_found()),
    }
  }

  fn write(
    &self,
    path: &Path,
    contents: &[u8],
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;
    let resolved = resolve(&nodes, &path)?;

    if nodes
      .get(&resolved)
      .is_some_and(super::node::Node::is_dir)
    {
      return Err(is_a_directory());
    }

    nodes.insert(resolved.clone(), Node::file(contents.to_vec()));
    materialize_parents(&mut nodes, &resolved);

    Ok(())
  }

  fn canonicalize(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf> {
    let nodes = self.files_read()?;
    let resolved = resolve(&nodes, &path.normalize())?;

    if !nodes.contains_key(&resolved) {
      return Err(not_found());
    }

    Ok(resolved)
  }

  fn read_link(
    &self,
    path: &Path,
  ) -> io::Result<PathBuf> {
    let path = path.normalize();
    let nodes = self.files_read()?;

    match nodes.get(&path) {
      Some(Node {
        kind: NodeKind::Symlink(target),
        ..
      }) => Ok(target.clone()),
      Some(_) => Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "not a symbolic link",
      )),
      None => Err(not_found()),
    }
  }

  fn copy(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<u64> {
    let mut nodes = self.files_write()?;
    let from = resolve(&nodes, &from.normalize())?;
    let to = resolve(&nodes, &to.normalize())?;

    if nodes.get(&from).is_some_and(super::node::Node::is_dir) {
      return Err(is_a_directory());
    }

    let contents = match nodes.get(&from) {
      Some(Node {
        kind: NodeKind::File(data),
        ..
      }) => data.read().map_err(|_| poisoned())?.clone(),
      Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
      None => return Err(not_found()),
    };

    nodes.insert(to.clone(), Node::file(contents.clone()));
    materialize_parents(&mut nodes, &to);

    Ok(contents.len() as u64)
  }

  fn create_dir(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;
    create_directories(&mut nodes, &path, false)
  }

  fn create_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;
    create_directories(&mut nodes, &path, true)
  }

  fn create_dir_with(
    &self,
    path: &Path,
    builder: &DirBuilder,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;
    create_directories(&mut nodes, &path, builder.get_recursive())?;

    #[cfg(unix)]
    if let Some(mode) = builder.get_mode()
      && let Some(node) = nodes.get_mut(&path)
    {
      node.permissions = Permissions::from_mode(mode);
    }

    Ok(())
  }

  fn hard_link(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()> {
    let original = original.normalize();
    let link = link.normalize();
    let mut nodes = self.files_write()?;

    let node = match nodes.get(&original) {
      Some(node) if node.is_dir() => {
        return Err(io::Error::new(
          io::ErrorKind::PermissionDenied,
          "cannot hard link a directory",
        ));
      }
      Some(node) => node.clone(),
      None => return Err(not_found()),
    };

    if nodes.contains_key(&link) {
      return Err(already_exists());
    }

    parent_is_directory(&nodes, &link)?;
    nodes.insert(link.clone(), node);
    materialize_parents(&mut nodes, &link);

    Ok(())
  }

  fn read_dir(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn ReadDir + Send + Sync>> {
    let path = path.normalize();
    let nodes = self.files_read()?;
    let resolved = resolve(&nodes, &path)?;

    match nodes.get(&resolved) {
      Some(node) if node.is_dir() => {}
      Some(_) => return Err(not_a_directory()),
      None => return Err(not_found()),
    }

    let mut entries: Vec<(PathBuf, MemoryMetadata)> = nodes
      .iter()
      .filter(|(entry, _)| entry.parent() == Some(resolved.as_path()))
      .map(|(entry, node)| (entry.clone(), node.metadata()))
      .collect();

    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let entries: Vec<io::Result<Box<dyn DirEntry + Send + Sync>>> = entries
      .into_iter()
      .map(|(entry, metadata)| {
        Ok(Box::new(MemoryDirEntry {
          path: entry,
          metadata,
        }) as Box<dyn DirEntry + Send + Sync>)
      })
      .collect();

    Ok(Box::new(entries.into_iter()))
  }

  fn remove_dir(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;

    match nodes.get(&path) {
      Some(node) if node.is_dir() => {}
      Some(_) => return Err(not_a_directory()),
      None => return Err(not_found()),
    }

    if nodes
      .keys()
      .any(|entry| entry.parent() == Some(path.as_path()))
    {
      return Err(io::Error::new(
        io::ErrorKind::DirectoryNotEmpty,
        "directory not empty",
      ));
    }

    nodes.remove(&path);

    Ok(())
  }

  fn remove_dir_all(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;

    match nodes.get(&path) {
      Some(node) if node.is_dir() => {}
      Some(_) => return Err(not_a_directory()),
      None => return Err(not_found()),
    }

    nodes.retain(|entry, _| !entry.starts_with(&path));

    Ok(())
  }

  fn remove_file(
    &self,
    path: &Path,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;

    match nodes.get(&path) {
      Some(node) if node.is_dir() => return Err(is_a_directory()),
      Some(_) => {}
      None => return Err(not_found()),
    }

    nodes.remove(&path);

    Ok(())
  }

  fn rename(
    &self,
    from: &Path,
    to: &Path,
  ) -> io::Result<()> {
    let from = from.normalize();
    let to = to.normalize();
    let mut nodes = self.files_write()?;

    if !nodes.contains_key(&from) {
      return Err(not_found());
    }

    let moving: Vec<(PathBuf, Node)> = nodes
      .iter()
      .filter(|(entry, _)| entry.starts_with(&from))
      .map(|(entry, node)| (entry.clone(), node.clone()))
      .collect();

    for (entry, _) in &moving {
      nodes.remove(entry);
    }

    for (entry, node) in moving {
      let suffix = entry
        .strip_prefix(&from)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();

      let destination = if suffix.as_os_str().is_empty() {
        to.clone()
      } else {
        to.join(suffix)
      };

      nodes.insert(destination, node);
    }

    materialize_parents(&mut nodes, &to);

    Ok(())
  }

  fn set_permissions(
    &self,
    path: &Path,
    perm: &Permissions,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;

    match nodes.get_mut(&path) {
      Some(node) => {
        node.permissions = *perm;
        Ok(())
      }
      None => Err(not_found()),
    }
  }

  fn set_times(
    &self,
    path: &Path,
    times: &FileTimes,
  ) -> io::Result<()> {
    let path = path.normalize();
    let mut nodes = self.files_write()?;

    match nodes.get_mut(&path) {
      Some(node) => {
        if let Some(accessed) = times.accessed() {
          node.accessed = accessed;
        }
        if let Some(modified) = times.modified() {
          node.modified = modified;
        }
        Ok(())
      }
      None => Err(not_found()),
    }
  }

  fn symlink(
    &self,
    original: &Path,
    link: &Path,
  ) -> io::Result<()> {
    let link = link.normalize();
    let mut nodes = self.files_write()?;

    if nodes.contains_key(&link) {
      return Err(already_exists());
    }

    parent_is_directory(&nodes, &link)?;
    nodes.insert(link.clone(), Node::symlink(original.to_path_buf()));
    materialize_parents(&mut nodes, &link);

    Ok(())
  }

  fn symlink_metadata(
    &self,
    path: &Path,
  ) -> io::Result<Box<dyn Metadata + Send + Sync>> {
    let path = path.normalize();
    let nodes = self.files_read()?;

    match nodes.get(&path) {
      Some(node) => Ok(Box::new(node.metadata())),
      None => Err(not_found()),
    }
  }

  fn try_exists(
    &self,
    path: &Path,
  ) -> io::Result<bool> {
    let path = path.normalize();
    let nodes = self.files_read()?;

    match resolve(&nodes, &path) {
      Ok(resolved) => Ok(nodes.contains_key(&resolved)),
      Err(_) => Ok(false),
    }
  }
}
