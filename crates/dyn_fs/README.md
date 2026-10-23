# Abstract FS API for Rust

This crate offers a trait that looks exactly like the `std::fs` interface and supplies implementations for the os filesystem and an in-memory filesystem.

Useful for substituting the FileSystem in tests and benchmarks

## Features

No features are enabled by default; opt in to the ones you need:

- `memory` — the in-memory `FileSystem` implementation at `dyn_fs::mem`.
- `async` — the asynchronous API at `dyn_fs::asynch`.
- `include_dir` — the read-only backend backed by assets embedded with
  [`include_dir`](https://crates.io/crates/include_dir) at `dyn_fs::include_dir`.

Enable them (the `memory` backend pulls in the `normalize-path` dependency, the
`async` backend pulls in `tokio`, and the `include_dir` backend pulls in the
`include_dir` dependency):

```toml
dyn_fs = { version = "...", features = ["memory", "async", "include_dir"] }
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
  let os_fs = OsFileSystem::new();
  let mem_fs = MemoryFileSystem::new();

  read_file(os_fs.as_ref());
  read_file(mem_fs.as_ref());
}
```

## Examples

One example crate per backend lives under `examples/`. Run any of them with
`cargo run -p <name>`:

| Example | Backend |
| --- | --- |
| `basic_sync` | `OsFileSystem` (synchronous, real OS) |
| `basic_sync_memory` | `MemoryFileSystem` (synchronous, in-memory) |
| `basic_sync_include_dir` | `IncludeDirFileSystem` (synchronous, read-only, embedded assets) |
| `basic_async_tokio` | `AsyncTokioFileSystem` (asynchronous, real OS) |
| `basic_async_memory` | `AsyncMemoryFileSystem` (asynchronous, in-memory) |
| `basic_async_include_dir` | `AsyncIncludeDirFileSystem` (asynchronous, read-only, embedded assets) |
