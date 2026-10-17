# Abstract FS API for Rust

This crate offers a trait that looks exactly like the `std::fs` interface and supplies implementations for the os filesystem and an in-memory filesystem.

Useful for substituting the FileSystem in tests and benchmarks

## Features

- `memory` — the in-memory `FileSystem` implementation at `dyn_fs::mem` (opt-in).
- `async` — the asynchronous API at `dyn_fs::asynch` (opt-in).

Enable them (the in-memory backend pulls in the `normalize-path` dependency):

```toml
dyn_fs = { version = "...", features = ["memory", "async"] }
```

```rust
use dyn_fs::FileSystem;
use dyn_fs::os::OsFileSystem;
use dyn_fs::mem::MemoryFileSystem;

fn read_file(fs: &dyn FileSystem) {
  let contents = fs.read(PathBuf::from("/path/to/file"))?;
  println!("{}", contents.len())
}

fn main() {
  let os_fs = OsFileSystem::default();
  let mem_fs = MemoryFileSystem::default();

  read_file(&os_fs);
  read_file(&mem_fs);
}
```
