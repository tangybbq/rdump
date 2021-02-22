// SPDX-License-Identifier: Apache-2.0
//! Backup actions.
//!
//! These actions can be assembled into an action sequence where some may
//! have setup and teardown aspects.  The runner will perform the teardowns
//! even if one of the later actions fail.

use anyhow::Result;

pub use borg::BorgBackup;
pub use snaps::{
    Stamp, LvmSnapshot, MountSnap, LvmRsure, SimpleRsure,
};
pub use runner::Runner;

mod borg;
mod runner;
mod snaps;

pub trait Action {
    fn perform(&mut self) -> Result<()>;
    fn cleanup(&mut self) -> Result<()>;

    /// Return a description of this action.
    fn describe(&self) -> String;
}

/// A very simple action that just prints a separator describing a block of
/// actions.
pub struct Message {
    text: String,
}

impl Message {
    pub fn new(text: &str) -> Result<Message> {
        Ok(Message{
            text: text.into(),
        })
    }
}

impl Action for Message {
    fn perform(&mut self) -> Result<()> {
        println!("------------------------------------------------------------");
        println!("    running: {}", self.text);
        println!("------------------------------------------------------------");
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    fn describe(&self) -> String {
        format!("    running: {}", self.text)
    }
}
