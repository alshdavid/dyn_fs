//! Demonstrates the synchronous, read-only [`IncludeDirFileSystem`].
//!
//! The `assets/` directory is embedded into the binary at compile time with
//! `include_dir`, so the file system is available without touching the disk.
//! Only the read-only parts of the interface are supported — every mutating
//! operation returns [`std::io::ErrorKind::Unsupported`].
//!
//! Run with:
//!
//! ```sh
//! cargo run --example basic_sync_include_dir
//! ```

use std::io;
use std::io::Read;
use std::path::Path;

use dyn_fs::FileSystem;
use dyn_fs::include_dir::IncludeDirFileSystem;

/// The embedded tree, rooted at this crate's `assets/` directory.
static ASSETS: include_dir::Dir<'static> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/assets");

fn main() -> io::Result<()> {
  // `Dir` is not `Copy`, so clone the (shallow) handle out of the static.
  let fs = IncludeDirFileSystem::new(ASSETS.clone());

  read(&fs)?;
  open(&fs)?;
  metadata(&fs)?;
  listing(&fs)?;
  existence(&fs)?;
  mutations_are_unsupported(&fs)?;

  println!();
  println!("done");

  Ok(())
}

/// Reads whole files as bytes and as strings.
fn read(fs: &dyn FileSystem) -> io::Result<()> {
  println!("-- read --");

  println!(
    "hello.txt:         {:?}",
    fs.read_to_string(Path::new("/hello.txt"))?
  );
  println!(
    "nested/info.txt:   {:?}",
    fs.read_to_string(Path::new("/nested/info.txt"))?
  );
  println!(
    "nested/deep/note:  {:?}",
    fs.read_to_string(Path::new("/nested/deep/note.md"))?
  );
  println!(
    "read() byte count: {}",
    fs.read(Path::new("/nested/info.txt"))?.len()
  );

  Ok(())
}

/// Opens a file and streams it through the standard `Read` trait.
fn open(fs: &dyn FileSystem) -> io::Result<()> {
  println!("-- open --");

  let mut file = fs.open(Path::new("/nested/info.txt"))?;
  let mut buffer = String::new();
  file.read_to_string(&mut buffer)?;

  println!("streamed via Read: {buffer:?}");

  Ok(())
}

/// Inspects metadata for files and directories.
fn metadata(fs: &dyn FileSystem) -> io::Result<()> {
  println!("-- metadata --");

  let file = fs.metadata(Path::new("/hello.txt"))?;
  println!("hello.txt  len: {}", file.len());
  println!("hello.txt  is_file: {}", file.is_file());
  println!("hello.txt  readonly: {}", file.permissions().readonly());

  let dir = fs.symlink_metadata(Path::new("/nested"))?;
  println!("nested/    is_dir: {}", dir.is_dir());

  Ok(())
}

/// Lists the root directory and a nested directory.
fn listing(fs: &dyn FileSystem) -> io::Result<()> {
  println!("-- read_dir --");

  println!("  /");
  for entry in fs.read_dir(Path::new("/"))? {
    let entry = entry?;
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
  for entry in fs.read_dir(Path::new("/nested"))? {
    let entry = entry?;
    println!("    {}", entry.path().display());
  }

  Ok(())
}

/// Checks for paths and canonicalizes them.
fn existence(fs: &dyn FileSystem) -> io::Result<()> {
  println!("-- existence / canonicalize --");

  println!(
    "/hello.txt exists:    {}",
    fs.try_exists(Path::new("/hello.txt"))?
  );
  println!(
    "/missing.txt exists:  {}",
    fs.try_exists(Path::new("/missing.txt"))?
  );
  println!(
    "canonicalize:         {:?}",
    fs.canonicalize(Path::new("/nested/info.txt"))?
  );

  Ok(())
}

/// Shows that mutating operations report `Unsupported`.
fn mutations_are_unsupported(fs: &dyn FileSystem) -> io::Result<()> {
  println!("-- mutations are unsupported --");

  let write = fs.write(Path::new("/hello.txt"), b"nope").unwrap_err();
  println!("write:       {:?}", write.kind());

  let create = fs.create_dir(Path::new("/new")).unwrap_err();
  println!("create_dir:  {:?}", create.kind());

  let remove = fs.remove_file(Path::new("/hello.txt")).unwrap_err();
  println!("remove_file: {:?}", remove.kind());

  Ok(())
}
