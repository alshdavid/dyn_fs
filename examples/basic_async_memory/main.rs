//! Demonstrates the asynchronous, in-memory [`AsyncMemoryFileSystem`].
//!
//! Unlike the `tokio` backend this implementation performs no blocking work and
//! spawns no tasks: every future resolves on its first poll. It is driven
//! through the same runtime-agnostic [`AsyncFileSystem`] trait, and its file
//! handles implement `tokio`'s `AsyncRead` / `AsyncWrite` / `AsyncSeek` traits.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example basic_async_memory
//! ```

use std::io;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;

use dyn_fs::OpenOptions;
use dyn_fs::asynch::AsyncFileSystem;
use dyn_fs::asynch::mem::AsyncMemoryFileSystem;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> io::Result<()> {
  let fs = AsyncMemoryFileSystem::new();

  directories(fs.as_ref()).await?;
  write_and_read(fs.as_ref()).await?;
  handle_io(fs.as_ref()).await?;
  listing(fs.as_ref()).await?;
  copy_rename_and_symlink(fs.as_ref()).await?;
  metadata(fs.as_ref()).await?;
  shared_state(&fs).await?;
  cleanup(fs.as_ref()).await?;

  println!();
  println!("done");

  Ok(())
}

async fn directories(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- directories --");

  fs.create_dir(Path::new("/notes")).await?;
  fs.create_dir_all(Path::new("/a/b/c")).await?;

  println!(
    "/notes is dir: {}",
    fs.metadata(Path::new("/notes")).await?.is_dir()
  );
  println!(
    "/a/b/c exists: {}",
    fs.try_exists(Path::new("/a/b/c")).await?
  );

  Ok(())
}

async fn write_and_read(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- write / read --");

  let file = Path::new("/notes/hello.txt");
  fs.write(file, b"hello world").await?;

  println!("read_to_string: {:?}", fs.read_to_string(file).await?);
  println!("read:           {} bytes", fs.read(file).await?.len());

  Ok(())
}

async fn handle_io(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- handle io --");

  let file = Path::new("/notes/hello.txt");

  let mut options = OpenOptions::new();
  options.write(true).append(true);

  let mut handle = fs.open_with(file, options).await?;
  handle.write_all(b"!!!").await?;
  handle.sync_all().await?;
  drop(handle);

  println!("after append: {:?}", fs.read_to_string(file).await?);

  let mut handle = fs.open(file).await?;
  handle.seek(SeekFrom::Start(6)).await?;
  let mut rest = String::new();
  handle.read_to_string(&mut rest).await?;

  println!("from offset 6: {rest:?}");

  // The handle itself exposes metadata and truncation.
  handle.set_len(6).await?;
  println!(
    "handle len after set_len: {}",
    handle.metadata().await?.len()
  );

  Ok(())
}

async fn listing(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- read_dir --");

  let mut read_dir = fs.read_dir(Path::new("/notes")).await?;

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

async fn copy_rename_and_symlink(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- copy / rename / symlink --");

  let original = Path::new("/notes/hello.txt");
  let copy = Path::new("/notes/hello.copy.txt");
  let renamed = Path::new("/notes/renamed.txt");

  let copied = fs.copy(original, copy).await?;
  println!("copied {copied} bytes");

  fs.rename(copy, renamed).await?;
  println!("renamed exists: {}", fs.try_exists(renamed).await?);

  fs.symlink(renamed, Path::new("/notes/link.txt")).await?;
  println!(
    "read_link: {:?}",
    fs.read_link(Path::new("/notes/link.txt")).await?
  );
  println!(
    "via link:  {:?}",
    fs.read_to_string(Path::new("/notes/link.txt")).await?
  );

  Ok(())
}

async fn metadata(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- metadata --");

  let file = Path::new("/notes/renamed.txt");
  let metadata = fs.metadata(file).await?;

  println!("len:      {}", metadata.len());
  println!("is_file:  {}", metadata.is_file());
  println!("is_empty: {}", metadata.is_empty());

  println!("canonicalize: {:?}", fs.canonicalize(file).await?);

  Ok(())
}

/// Clones of the shared handle point at the same underlying file system.
async fn shared_state(fs: &Arc<dyn AsyncFileSystem>) -> io::Result<()> {
  println!("-- shared state --");

  let clone = Arc::clone(fs);
  clone
    .write(Path::new("/shared.txt"), b"written through clone")
    .await?;

  println!(
    "visible via original: {:?}",
    fs.read_to_string(Path::new("/shared.txt")).await?
  );

  Ok(())
}

async fn cleanup(fs: &dyn AsyncFileSystem) -> io::Result<()> {
  println!("-- remove --");

  fs.create_dir(Path::new("/empty")).await?;
  fs.remove_dir(Path::new("/empty")).await?;
  println!(
    "/empty exists: {}",
    fs.try_exists(Path::new("/empty")).await?
  );

  fs.remove_file(Path::new("/shared.txt")).await?;
  println!(
    "/shared.txt exists: {}",
    fs.exists(Path::new("/shared.txt")).await
  );

  fs.remove_dir_all(Path::new("/notes")).await?;
  println!(
    "/notes exists: {}",
    fs.try_exists(Path::new("/notes")).await?
  );

  Ok(())
}
