use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

use normalize_path::NormalizePath;

use super::AsyncMemoryDirEntry;
use super::AsyncMemoryFile;
use super::AsyncMemoryMetadata;
use super::AsyncMemoryReadDir;
use super::node::AsyncNode;
use super::node::AsyncNodeKind;
use super::node::AsyncNodes;
use crate::asynch::file_system::AsyncDirEntry;
use crate::asynch::file_system::AsyncFile;
use crate::asynch::file_system::AsyncFileSystem;
use crate::asynch::file_system::AsyncMetadata;
use crate::asynch::file_system::AsyncReadDir;
use crate::asynch::file_system::BoxFuture;
use crate::asynch::file_system::DirBuilder;
use crate::asynch::file_system::FileTimes;
use crate::asynch::file_system::OpenOptions;
use crate::asynch::file_system::Permissions;

/// An in-memory, thread-safe implementation of [`AsyncFileSystem`].
///
/// The file system is a flat table of normalized paths mapping to nodes. Every
/// mutating operation routes through the shared table, so clones of a
/// `MemoryFileSystem` observe the same data.
#[derive(Default, Debug, Clone)]
pub struct AsyncMemoryFileSystem {
  files: Arc<RwLock<AsyncNodes>>,
}

impl AsyncMemoryFileSystem {
  /// Creates an empty in-memory file system, returned as a shared [`Arc`] trait
  /// object.
  #[allow(clippy::new_ret_no_self)]
  pub fn new() -> Arc<dyn AsyncFileSystem> {
    Arc::new(Self::default())
  }

  fn files_write<'a>(&'a self) -> io::Result<RwLockWriteGuard<'a, AsyncNodes>> {
    let Ok(files) = self.files.write() else {
      return Err(io::Error::other("memory file system lock poisoned"));
    };
    Ok(files)
  }

  fn files_read<'a>(&'a self) -> io::Result<RwLockReadGuard<'a, AsyncNodes>> {
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
  nodes: &mut AsyncNodes,
  path: &Path,
) {
  let mut parent = path.parent();
  while let Some(directory) = parent {
    nodes
      .entry(directory.to_path_buf())
      .or_insert_with(AsyncNode::directory);

    if directory.as_os_str().is_empty() {
      break;
    }

    parent = directory.parent();
  }
}

/// Returns an error unless the parent of `path` exists and is a directory.
fn parent_is_directory(
  nodes: &AsyncNodes,
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
  nodes: &AsyncNodes,
  path: &Path,
) -> io::Result<PathBuf> {
  let mut path = path.normalize();
  let mut budget = 40;

  loop {
    let mut current = PathBuf::new();
    let mut link = None;

    for component in path.components() {
      current.push(component);

      if let Some(AsyncNode {
        kind: AsyncNodeKind::Symlink(target),
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
  nodes: &mut AsyncNodes,
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
    nodes.insert(path.to_path_buf(), AsyncNode::directory());
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
    nodes.insert(directory, AsyncNode::directory());
  }

  Ok(())
}

impl AsyncFileSystem for AsyncMemoryFileSystem {
  fn open_with<'a>(
    &'a self,
    path: &'a Path,
    options: OpenOptions,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncFile + Send + Sync>>> {
    Box::pin(async move {
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
          AsyncNodeKind::File(data) => Some(Arc::clone(data)),
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
            nodes.insert(resolved.clone(), AsyncNode::file(Vec::new()));
            materialize_parents(&mut nodes, &resolved);
          } else {
            return Err(not_found());
          }
        }
      }

      drop(nodes);

      Ok(Box::new(AsyncMemoryFile {
        files: Arc::clone(&self.files),
        path: resolved,
        position: 0,
        append: options.get_append(),
      }) as Box<dyn AsyncFile + Send + Sync>)
    })
  }

  fn read<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Vec<u8>>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;
      let resolved = resolve(&nodes, &path)?;

      match nodes.get(&resolved) {
        Some(AsyncNode {
          kind: AsyncNodeKind::File(data),
          ..
        }) => {
          let data = data.read().map_err(|_| poisoned())?;
          Ok(data.clone())
        }
        Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
        None => Err(not_found()),
      }
    })
  }

  fn read_to_string<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<String>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;
      let resolved = resolve(&nodes, &path)?;

      let bytes = match nodes.get(&resolved) {
        Some(AsyncNode {
          kind: AsyncNodeKind::File(data),
          ..
        }) => {
          let data = data.read().map_err(|_| poisoned())?;
          data.clone()
        }
        Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
        None => return Err(not_found()),
      };

      String::from_utf8(bytes)
        .map_err(|_| io::Error::other("unable to read file as a UTF-8 string"))
    })
  }

  fn metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;
      let resolved = resolve(&nodes, &path)?;

      match nodes.get(&resolved) {
        Some(node) => Ok(Box::new(node.metadata()) as Box<dyn AsyncMetadata + Send + Sync>),
        None => Err(not_found()),
      }
    })
  }

  fn write<'a>(
    &'a self,
    path: &'a Path,
    contents: &'a [u8],
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let path = path.normalize();
      let mut nodes = self.files_write()?;
      let resolved = resolve(&nodes, &path)?;

      if nodes
        .get(&resolved)
        .map(|node| node.is_dir())
        .unwrap_or(false)
      {
        return Err(is_a_directory());
      }

      nodes.insert(resolved.clone(), AsyncNode::file(contents.to_vec()));
      materialize_parents(&mut nodes, &resolved);

      Ok(())
    })
  }

  fn canonicalize<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>> {
    Box::pin(async move {
      let nodes = self.files_read()?;
      let resolved = resolve(&nodes, &path.normalize())?;

      if !nodes.contains_key(&resolved) {
        return Err(not_found());
      }

      Ok(resolved)
    })
  }

  fn read_link<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<PathBuf>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;

      match nodes.get(&path) {
        Some(AsyncNode {
          kind: AsyncNodeKind::Symlink(target),
          ..
        }) => Ok(target.clone()),
        Some(_) => Err(io::Error::new(
          io::ErrorKind::InvalidInput,
          "not a symbolic link",
        )),
        None => Err(not_found()),
      }
    })
  }

  fn copy<'a>(
    &'a self,
    from: &'a Path,
    to: &'a Path,
  ) -> BoxFuture<'a, io::Result<u64>> {
    Box::pin(async move {
      let mut nodes = self.files_write()?;
      let from = resolve(&nodes, &from.normalize())?;
      let to = resolve(&nodes, &to.normalize())?;

      if nodes.get(&from).map(|node| node.is_dir()).unwrap_or(false) {
        return Err(is_a_directory());
      }

      let contents = match nodes.get(&from) {
        Some(AsyncNode {
          kind: AsyncNodeKind::File(data),
          ..
        }) => data.read().map_err(|_| poisoned())?.clone(),
        Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
        None => return Err(not_found()),
      };

      nodes.insert(to.clone(), AsyncNode::file(contents.clone()));
      materialize_parents(&mut nodes, &to);

      Ok(contents.len() as u64)
    })
  }

  fn create_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let path = path.normalize();
      let mut nodes = self.files_write()?;
      create_directories(&mut nodes, &path, false)
    })
  }

  fn create_dir_all<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let path = path.normalize();
      let mut nodes = self.files_write()?;
      create_directories(&mut nodes, &path, true)
    })
  }

  fn create_dir_with<'a>(
    &'a self,
    path: &'a Path,
    builder: DirBuilder,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
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
    })
  }

  fn hard_link<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
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
    })
  }

  fn read_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncReadDir>>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;
      let resolved = resolve(&nodes, &path)?;

      match nodes.get(&resolved) {
        Some(node) if node.is_dir() => {}
        Some(_) => return Err(not_a_directory()),
        None => return Err(not_found()),
      }

      let mut entries: Vec<(PathBuf, AsyncMemoryMetadata)> = nodes
        .iter()
        .filter(|(entry, _)| entry.parent() == Some(resolved.as_path()))
        .map(|(entry, node)| (entry.clone(), node.metadata()))
        .collect();

      entries.sort_by(|(left, _), (right, _)| left.cmp(right));

      let entries: Vec<Box<dyn AsyncDirEntry + Send + Sync>> = entries
        .into_iter()
        .map(|(entry, metadata)| {
          Box::new(AsyncMemoryDirEntry {
            path: entry,
            metadata,
          }) as Box<dyn AsyncDirEntry + Send + Sync>
        })
        .collect();

      Ok(Box::new(AsyncMemoryReadDir::from(entries)) as Box<dyn AsyncReadDir>)
    })
  }

  fn remove_dir<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
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
    })
  }

  fn remove_dir_all<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let path = path.normalize();
      let mut nodes = self.files_write()?;

      match nodes.get(&path) {
        Some(node) if node.is_dir() => {}
        Some(_) => return Err(not_a_directory()),
        None => return Err(not_found()),
      }

      nodes.retain(|entry, _| !entry.starts_with(&path));

      Ok(())
    })
  }

  fn remove_file<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let path = path.normalize();
      let mut nodes = self.files_write()?;

      match nodes.get(&path) {
        Some(node) if node.is_dir() => return Err(is_a_directory()),
        Some(_) => {}
        None => return Err(not_found()),
      }

      nodes.remove(&path);

      Ok(())
    })
  }

  fn rename<'a>(
    &'a self,
    from: &'a Path,
    to: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let from = from.normalize();
      let to = to.normalize();
      let mut nodes = self.files_write()?;

      if !nodes.contains_key(&from) {
        return Err(not_found());
      }

      let moving: Vec<(PathBuf, AsyncNode)> = nodes
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
          .map(|path| path.to_path_buf())
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
    })
  }

  fn set_permissions<'a>(
    &'a self,
    path: &'a Path,
    perm: Permissions,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let path = path.normalize();
      let mut nodes = self.files_write()?;

      match nodes.get_mut(&path) {
        Some(node) => {
          node.permissions = perm;
          Ok(())
        }
        None => Err(not_found()),
      }
    })
  }

  fn set_times<'a>(
    &'a self,
    path: &'a Path,
    times: FileTimes,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
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
    })
  }

  fn symlink<'a>(
    &'a self,
    original: &'a Path,
    link: &'a Path,
  ) -> BoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
      let link = link.normalize();
      let mut nodes = self.files_write()?;

      if nodes.contains_key(&link) {
        return Err(already_exists());
      }

      parent_is_directory(&nodes, &link)?;
      nodes.insert(link.clone(), AsyncNode::symlink(original.to_path_buf()));
      materialize_parents(&mut nodes, &link);

      Ok(())
    })
  }

  fn symlink_metadata<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<Box<dyn AsyncMetadata + Send + Sync>>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;

      match nodes.get(&path) {
        Some(node) => Ok(Box::new(node.metadata()) as Box<dyn AsyncMetadata + Send + Sync>),
        None => Err(not_found()),
      }
    })
  }

  fn try_exists<'a>(
    &'a self,
    path: &'a Path,
  ) -> BoxFuture<'a, io::Result<bool>> {
    Box::pin(async move {
      let path = path.normalize();
      let nodes = self.files_read()?;

      match resolve(&nodes, &path) {
        Ok(resolved) => Ok(nodes.contains_key(&resolved)),
        Err(_) => Ok(false),
      }
    })
  }
}
