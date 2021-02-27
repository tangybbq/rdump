// SPDX-License-Identifier: Apache-2.0
//! Actions related to ZFS.
//!
//! These actions are useful when mirroring from regular filesystems to ZFS
//! filesystems.

use anyhow::Result;
use log::info;
use std::process::Command;

use super::Action;
use crate::checked::CheckedExt;

static ZFS: &'static str = "/usr/sbin/zfs";
static RSYNC: &'static str = "/usr/bin/rsync";

/// An action that rsyncs from a mounted snapshot to a zfs target.
pub struct Rsync {
    src: String,
    dest: String,
    acls: bool,
    verbose: bool,
}

impl Rsync {
    pub fn new(src: &str, dest: &str, acls: bool, verbose: bool) -> Result<Rsync> {
        Ok(Rsync {
            src: src.into(),
            dest: dest.into(),
            acls: acls,
            verbose: verbose,
        })
    }
}

impl Action for Rsync {
    fn perform(&mut self) -> Result<()> {
        info!("Rsyncing from {} to {}", self.src, self.dest);
        let mut cmd = Command::new(RSYNC);
        cmd.args(&["-aHx", "--delete"]);
        if self.verbose {
            cmd.arg("-i");
        }
        if self.acls {
            cmd.arg("-AX");
        }
        cmd.arg(&format!("{}/.", self.src));
        cmd.arg(&format!("{}/.", self.dest));
        cmd.checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        // No cleanup.
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Rsync from {} to {}", self.src, self.dest)
    }
}

/// An action that creates a ZFS snapshot.
pub struct ZfsSnapshot {
    volume: String,
    snap: String,
}

impl ZfsSnapshot {
    pub fn new(volume: &str, snap: &str) -> Result<ZfsSnapshot> {
        Ok(ZfsSnapshot {
            volume: volume.into(),
            snap: snap.into(),
        })
    }
}

impl Action for ZfsSnapshot {
    fn perform(&mut self) -> Result<()> {
        let snap = format!("{}@{}", self.volume, self.snap);
        info!("Zfs snapshot {}", snap);
        Command::new(ZFS)
            .args(&["snapshot", &snap])
            .checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        // No cleanup.
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Zfs snapshot {}@{}", self.volume, self.snap)
    }
}
