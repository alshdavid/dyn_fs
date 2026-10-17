use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::asynch::file_system::AsyncPermissions;

impl From<fs::Permissions> for AsyncPermissions {
  fn from(value: fs::Permissions) -> Self {
    #[cfg(unix)]
    {
      AsyncPermissions::from_mode(value.mode())
    }
    #[cfg(not(unix))]
    {
      AsyncPermissions::from_readonly(value.readonly())
    }
  }
}
