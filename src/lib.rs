//! Main body of code.

pub use anyhow::Result;

pub use zfs::Zfs;
pub use sudo::Sudo;

mod checked;
mod sudo;
mod zfs;
