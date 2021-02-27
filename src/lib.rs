// SPDX-License-Identifier: Apache-2.0
//! Main body of code.

pub use anyhow::Result;

pub use checked::CheckedExt;
pub use config::ConfigFile;
pub use sudo::Sudo;
pub use zfs::Zfs;

pub mod actions;
mod checked;
pub mod config;
mod sudo;
mod zfs;
