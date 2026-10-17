//! Read-only asynchronous file system backed by assets embedded with `include_dir`
//!
//! [`AsyncIncludeDirFileSystem`] mounts an [`include_dir::Dir`] as an
//! [`AsyncFileSystem`], exposing the read-only portions of the interface
//! (reads, metadata, directory listing and existence checks). Operations that
//! would mutate the embedded data return [`std::io::ErrorKind::Unsupported`].
//!
//! Every operation completes synchronously (the data is in memory), so the
//! returned futures resolve on their first poll without spawning any work.
//!
//! [`AsyncFileSystem`]: crate::asynch::file_system::AsyncFileSystem

mod dir_entry;
mod file;
mod file_system;
mod file_type;
mod metadata;
mod read_dir;

pub use self::dir_entry::*;
pub use self::file::*;
pub use self::file_system::*;
pub use self::file_type::*;
pub use self::metadata::*;
pub use self::read_dir::*;
