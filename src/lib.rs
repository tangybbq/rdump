//! Main body of code.

pub use anyhow::Result;

pub use zfs::Zfs;
pub use sudo::Sudo;
pub use checked::CheckedExt;
pub use config::ConfigFile;

pub mod actions;
mod checked;
pub mod config;
mod sudo;
mod zfs;
