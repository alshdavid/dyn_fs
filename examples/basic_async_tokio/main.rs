//! Demonstrates the asynchronous, `tokio`-backed [`AsyncTokioFileSystem`].
//!
//! This backend forwards every operation to `tokio::fs`, so it reflects the real
//! operating-system file system. Like the other asynchronous backends it is
//! driven through the runtime-agnostic [`AsyncFileSystem`] trait, and the file
//! handles it returns implement `tokio`'s own `AsyncRead` / `AsyncWrite` /
//! `AsyncSeek` traits.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example basic_async_tokio
//! ```

use std::io;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;

use dyn_fs::OpenOptions;
use dyn_fs::asynch::AsyncFileSystem;
use dyn_fs::asynch::tokio::AsyncTokioFileSystem;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> io::Result<()> {
  // Work inside a scratch directory under the system temp dir so we do not
  // touch the user's files.
  let root = std::env::temp_dir().join("dyn_fs_basic_async_tokio");
  let fs = AsyncTokioFileSystem::new();

  let _ = fs.remove_dir_all(&root).await;

  let result = run(fs.as_ref(), &root).await;

  let _ = fs.remove_dir_all(&root).await;
  result
}

async fn run(
  fs: &dyn AsyncFileSystem,
  root: &Path,
) -> io::Result<()> {
  println!("root: {root:?}");
  println!();

  directories(fs, root).await?;
  write_and_read(fs, root).await?;
  handle_io(fs, root).await?;
  listing(fs, root).await?;
  metadata_and_permissions(fs, root).await?;

  println!();
  println!("done");

  Ok(())
}

async fn directories(
  fs: &dyn AsyncFileSystem,
  root: &Path,
) -> io::Result<()> {
  println!("-- directories --");

  fs.create_dir_all(&root.join("a/b/c")).await?;
  println!(
    "a/b/c exists: {}",
    fs.try_exists(&root.join("a/b/c")).await?
  );
  println!(
    "a/b/c is dir: {}",
    fs.metadata(&root.join("a/b/c")).await?.is_dir()
  );

  Ok(())
}

async fn write_and_read(
  fs: &dyn AsyncFileSystem,
  root: &Path,
) -> io::Result<()> {
  println!("-- write / read --");

  let file = root.join("hello.txt");
  fs.write(&file, b"hello world").await?;

  println!("read_to_string: {:?}", fs.read_to_string(&file).await?);
  println!("read:           {} bytes", fs.read(&file).await?.len());

  Ok(())
}

/// Exercises the `tokio` IO traits on the returned file handles.
async fn handle_io(
  fs: &dyn AsyncFileSystem,
  root: &Path,
) -> io::Result<()> {
  println!("-- handle io --");

  let file = root.join("hello.txt");

  let mut options = OpenOptions::new();
  options.write(true).append(true);

  let mut handle = fs.open_with(&file, options).await?;
  handle.write_all(b"!!!").await?;
  handle.sync_all().await?;
  drop(handle);

  println!("after append: {:?}", fs.read_to_string(&file).await?);

  let mut handle = fs.open(&file).await?;
  handle.seek(SeekFrom::Start(6)).await?;
  let mut rest = String::new();
  handle.read_to_string(&mut rest).await?;

  println!("from offset 6: {rest:?}");

  Ok(())
}

async fn listing(
  fs: &dyn AsyncFileSystem,
  root: &Path,
) -> io::Result<()> {
  println!("-- read_dir --");

  let mut read_dir = fs.read_dir(root).await?;

  while let Some(entry) = read_dir.next_entry().await? {
    let kind = if entry.file_type()?.is_dir() {
      "dir "
    } else {
      "file"
    };
    println!("  {kind} {}", entry.file_name().to_string_lossy());
  }

  Ok(())
}

async fn metadata_and_permissions(
  fs: &dyn AsyncFileSystem,
  root: &Path,
) -> io::Result<()> {
  println!("-- metadata / permissions --");

  let file = root.join("hello.txt");
  let metadata = fs.metadata(&file).await?;

  println!("len:      {}", metadata.len());
  println!("is_file:  {}", metadata.is_file());
  println!("is_empty: {}", metadata.is_empty());
  println!("readonly: {}", metadata.permissions().readonly());

  println!("canonicalize: {:?}", fs.canonicalize(&file).await?);

  // A path that does not exist.
  let missing: PathBuf = root.join("missing.txt");
  println!("missing exists: {}", fs.exists(&missing).await);

  Ok(())
}
