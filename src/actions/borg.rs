//! Actions related to borg backup

use anyhow::Result;
use log::info;
use std::{
    process::Command,
};

use crate::checked::CheckedExt;
use super::Action;

/// An action that performs a borg backup.  This needs a path to a borg
/// invoking script (TODO: We pass the passwords through this way, but
/// maybe they should be in the config file).
pub struct BorgBackup {
    /// The directory of the snapshot.
    snap: String,

    /// The borg script to run.
    script: String,

    /// The name appended to the backup.
    name: String,
}

impl BorgBackup {
    pub fn new(snap: &str, script: &str, name: &str) -> Result<BorgBackup> {
        Ok(BorgBackup {
            snap: snap.into(),
            script: script.into(),
            name: name.into(),
        })
    }
}

impl Action for BorgBackup {
    fn perform(&mut self) -> Result<()> {
        info!("Running borg backup of {} via {}", self.snap, self.name);
        Command::new(&self.script)
            .args(&["create", "--exclude-caches", "-x", "--stat",
                "--progress",
                &format!("::{}", self.name),
                &self.snap])
            .checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        // No cleanup.
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Borg backup of {} to {}", self.snap, self.name)
    }
}
