//! Demonstrates the asynchronous, read-only [`AsyncIncludeDirFileSystem`].
//!
//! Like the asynchronous in-memory backend every operation completes
//! synchronously under the hood (the data is embedded in the binary), so the
//! returned futures resolve on their first poll without spawning any work. The
//! interface is still fully asynchronous, so it composes with the other
//! [`AsyncFileSystem`] backends.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example basic_async_include_dir
//! ```

use std::io;
use std::path::Path;

use dyn_fs::asynch::AsyncFileSystem;
use dyn_fs::asynch::include_dir::AsyncIncludeDirFileSystem;
use tokio::io::AsyncReadExt;

/// The embedded tree, rooted at this crate's `assets/` directory.
static ASSETS: include_dir::Dir<'static> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/assets");

#[tokio::main]
async fn main() -> io::Result<()> {
  // `Dir` is not `Copy`, so clone the (shallow) handle out of the static.
  let fs = AsyncIncludeDirFileSystem::new(ASSETS.clone());

  read(&fs).await?;
  handle_io(&fs).await?;
  metadata(&fs).await?;
  listing(&fs).await?;
  mutations_are_unsupported(&fs).await?;

  println!();
  println!("done");

  Ok(())
}

async fn read<F: AsyncFileSystem>(fs: &F) -> io::Result<()> {
  println!("-- read --");

  println!(
    "hello.txt:        {:?}",
    fs.read_to_string(Path::new("/hello.txt")).await?
  );
  println!(
    "nested/info.txt:  {:?}",
    fs.read_to_string(Path::new("/nested/info.txt")).await?
  );
  println!(
    "read() byte count: {}",
    fs.read(Path::new("/nested/info.txt")).await?.len()
  );

  Ok(())
}

async fn handle_io<F: AsyncFileSystem>(fs: &F) -> io::Result<()> {
  println!("-- handle io --");

  let mut handle = fs.open(Path::new("/nested/deep/note.md")).await?;
  let mut contents = String::new();
  handle.read_to_string(&mut contents).await?;

  println!("streamed via AsyncRead: {contents:?}");

  Ok(())
}

async fn metadata<F: AsyncFileSystem>(fs: &F) -> io::Result<()> {
  println!("-- metadata --");

  let file = fs.metadata(Path::new("/hello.txt")).await?;
  println!("hello.txt len:      {}", file.len());
  println!("hello.txt is_file:  {}", file.is_file());
  println!("hello.txt readonly: {}", file.permissions().readonly());

  let dir = fs.symlink_metadata(Path::new("/nested")).await?;
  println!("nested is_dir:      {}", dir.is_dir());

  Ok(())
}

async fn listing<F: AsyncFileSystem>(fs: &F) -> io::Result<()> {
  println!("-- read_dir --");

  println!("  /");
  let mut read_dir = fs.read_dir(Path::new("/")).await?;
  while let Some(entry) = read_dir.next_entry().await? {
    println!(
      "    {} {}",
      if entry.file_type()?.is_dir() {
        "dir "
      } else {
        "file"
      },
      entry.file_name().to_string_lossy()
    );
  }

  println!("  /nested");
  let mut read_dir = fs.read_dir(Path::new("/nested")).await?;
  while let Some(entry) = read_dir.next_entry().await? {
    println!("    {}", entry.path().display());
  }

  println!(
    "canonicalize: {:?}",
    fs.canonicalize(Path::new("/nested/info.txt")).await?
  );

  Ok(())
}

async fn mutations_are_unsupported<F: AsyncFileSystem>(fs: &F) -> io::Result<()> {
  println!("-- mutations are unsupported --");

  let write = fs
    .write(Path::new("/hello.txt"), b"nope")
    .await
    .unwrap_err();
  println!("write:       {:?}", write.kind());

  let create = fs.create_dir(Path::new("/new")).await.unwrap_err();
  println!("create_dir:  {:?}", create.kind());

  let remove = fs.remove_file(Path::new("/hello.txt")).await.unwrap_err();
  println!("remove_file: {:?}", remove.kind());

  Ok(())
}
