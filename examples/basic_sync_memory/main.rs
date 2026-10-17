//! Demonstrates the synchronous, in-memory [`MemoryFileSystem`].
//!
//! Every operation is exercised through the abstract [`FileSystem`] trait, so
//! the same code would work against any backend (the OS, `include_dir`, ...).
//!
//! Run with:
//!
//! ```sh
//! cargo run --example basic_sync_memory
//! ```

use std::io::Write;
use std::path::Path;

use dyn_fs::FileSystem;
use dyn_fs::OpenOptions;
use dyn_fs::ext::FileSystemExt;
use dyn_fs::mem::MemoryFileSystem;

fn main() -> std::io::Result<()> {
  let fs = MemoryFileSystem::default();

  directories(&fs)?;
  write_and_read(&fs)?;
  listing(&fs)?;
  copy_rename_and_symlink(&fs)?;
  open_with(&fs)?;
  metadata_and_permissions(&fs)?;
  ancestor_search(&fs)?;
  shared_state(&fs)?;
  cleanup(&fs)?;

  println!();
  println!("done");

  Ok(())
}

/// Creates directories, including nested paths.
fn directories(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- directories --");

  fs.create_dir(Path::new("/notes"))?;
  fs.create_dir_all(Path::new("/a/b/c"))?;

  println!(
    "/notes is dir: {}",
    fs.metadata(Path::new("/notes"))?.is_dir()
  );
  println!("/a/b/c exists: {}", fs.try_exists(Path::new("/a/b/c"))?);

  // Creating a directory that already exists is an error, unless recursive.
  let error = fs
    .create_dir(Path::new("/notes"))
    .err()
    .map(|error| error.kind());
  println!("duplicate create_dir: {error:?}");

  fs.create_dir_all(Path::new("/notes"))?;
  println!("recursive create of existing dir: ok");

  Ok(())
}

/// Writes files and reads them back as bytes and as a string.
fn write_and_read(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- write / read --");

  let file = Path::new("/notes/hello.txt");
  fs.write(file, b"hello world")?;
  println!("wrote {file:?}");

  let contents = fs.read_to_string(file)?;
  println!("read_to_string: {contents:?}");

  let bytes = fs.read(file)?;
  println!("read:           {} bytes", bytes.len());

  // Writing to a path materializes any missing ancestor directories.
  let deep = Path::new("/auto/created/parents.txt");
  fs.write(deep, b"parents were materialized")?;
  println!(
    "auto-created parents: {}",
    fs.metadata(Path::new("/auto/created"))?.is_dir()
  );

  Ok(())
}

/// Iterates over the entries of a directory.
fn listing(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- read_dir --");

  let mut entries = vec![];

  for entry in fs.read_dir(Path::new("/notes"))? {
    let entry = entry?;
    let kind = if entry.file_type()?.is_dir() {
      "dir "
    } else {
      "file"
    };
    entries.push(format!("{kind} {}", entry.file_name().to_string_lossy()));
  }

  entries.sort();

  for entry in entries {
    println!("  {entry}");
  }

  Ok(())
}

/// Copies, renames and creates symbolic links.
fn copy_rename_and_symlink(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- copy / rename / symlink --");

  let original = Path::new("/notes/hello.txt");
  let copy = Path::new("/notes/hello.copy.txt");
  let renamed = Path::new("/notes/renamed.txt");

  let copied = fs.copy(original, copy)?;
  println!("copied {copied} bytes -> {copy:?}");

  fs.rename(copy, renamed)?;
  println!(
    "renamed -> {renamed:?}, exists: {}",
    fs.try_exists(renamed)?
  );

  fs.symlink(renamed, Path::new("/notes/link.txt"))?;
  println!(
    "read_link: {:?}",
    fs.read_link(Path::new("/notes/link.txt"))?
  );
  println!(
    "via link:  {:?}",
    fs.read_to_string(Path::new("/notes/link.txt"))?
  );

  Ok(())
}

/// Opens a file with explicit options and appends to it.
fn open_with(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- open_with --");

  let file = Path::new("/notes/hello.txt");

  let mut options = OpenOptions::new();
  options.write(true).append(true);

  let mut handle = fs.open_with(file, &options)?;
  handle.write_all(b"!!!")?;
  handle.sync_all()?;
  drop(handle);

  println!("after append: {:?}", fs.read_to_string(file)?);

  Ok(())
}

/// Reads metadata and changes file permissions.
fn metadata_and_permissions(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- metadata / permissions --");

  let file = Path::new("/notes/hello.txt");
  let metadata = fs.metadata(file)?;

  println!("len:      {}", metadata.len());
  println!("is_file:  {}", metadata.is_file());
  println!("is_empty: {}", metadata.is_empty());

  let mut permissions = metadata.permissions();
  permissions.set_readonly(true);
  fs.set_permissions(file, &permissions)?;

  let readonly = fs.metadata(file)?.permissions().readonly();
  println!("readonly: {readonly}");

  Ok(())
}

/// Searches for a file by name in a directory and all of its parents.
fn ancestor_search<F: FileSystem>(fs: &F) -> std::io::Result<()> {
  println!("-- find_ancestor_file --");

  fs.write(Path::new("/config.toml"), b"[example]\n")?;

  let found = fs.find_ancestor_file(Path::new("/a/b/c"), "config.toml")?;
  println!("found: {found:?}");

  Ok(())
}

/// Clones of a `MemoryFileSystem` share the same underlying data.
fn shared_state(fs: &MemoryFileSystem) -> std::io::Result<()> {
  println!("-- shared state --");

  let clone = fs.clone();
  clone.write(Path::new("/shared.txt"), b"written through the clone")?;

  println!(
    "visible via original: {:?}",
    fs.read_to_string(Path::new("/shared.txt"))?
  );

  Ok(())
}

/// Removes files and directories.
fn cleanup(fs: &dyn FileSystem) -> std::io::Result<()> {
  println!("-- remove --");

  fs.create_dir(Path::new("/empty"))?;
  fs.remove_dir(Path::new("/empty"))?;
  println!("/empty exists: {}", fs.try_exists(Path::new("/empty"))?);

  fs.remove_file(Path::new("/shared.txt"))?;
  println!(
    "/shared.txt exists: {}",
    fs.exists(Path::new("/shared.txt"))
  );

  fs.remove_dir_all(Path::new("/notes"))?;
  println!("/notes exists: {}", fs.try_exists(Path::new("/notes"))?);

  fs.remove_dir_all(Path::new("/auto"))?;
  println!("/auto exists:  {}", fs.try_exists(Path::new("/auto"))?);

  Ok(())
}
