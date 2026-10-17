//! Read-only file system backed by assets embedded with `include_dir`
//!
//! [`IncludeDirFileSystem`] mounts an [`include_dir::Dir`] as a [`FileSystem`],
//! exposing the read-only portions of the interface (reads, metadata, directory
//! listing and existence checks). Operations that would mutate the embedded
//! data return [`std::io::ErrorKind::Unsupported`].
//!
//! [`FileSystem`]: crate::file_system::FileSystem

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
