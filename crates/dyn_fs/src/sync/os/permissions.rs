use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::file_system::Permissions;

impl From<fs::Permissions> for Permissions {
  fn from(value: fs::Permissions) -> Self {
    #[cfg(unix)]
    {
      Permissions::from_mode(value.mode())
    }
    #[cfg(not(unix))]
    {
      Permissions::from_readonly(value.readonly())
    }
  }
}
