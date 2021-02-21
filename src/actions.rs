//! Backup actions.
//!
//! These actions can be assembled into an action sequence where some may
//! have setup and teardown aspects.  The runner will perform the teardowns
//! even if one of the later actions fail.

use anyhow::Result;

pub use borg::BorgBackup;
pub use snaps::{
    Stamp, LvmSnapshot, MountSnap, LvmRsure,
};

mod borg;
mod snaps;

pub trait Action {
    fn perform(&mut self) -> Result<()>;
    fn cleanup(&mut self) -> Result<()>;
}
