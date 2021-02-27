// SPDX-License-Identifier: Apache-2.0
//! ZFS management for testing.
//!
//! We will create real ZFS volumes under a test volume and then destroy
//! the whole thing when we are finished.

use rdump::CheckedExt;

use anyhow::Result;
use std::process::Command;

static ZFS_BASE: &'static str = "lint/fstest";
static ZFS: &'static str = "/usr/sbin/zfs";

pub struct ZfsTest {
    base: String,
}

impl ZfsTest {
    /// Set up a new test volume, with potential subvolumes to be created.
    pub fn setup() -> Result<ZfsTest> {
        log::info!("Creating zfs test volume: {}", ZFS_BASE);
        Command::new(ZFS)
            .args(&["create", ZFS_BASE])
            .checked_noio()?;

        Ok(ZfsTest {
            base: ZFS_BASE.into(),
        })
    }

    /// Cleanup the ZFS test volumes.
    pub fn cleanup(&self) -> Result<()> {
        // Use -r for the cleanup, so we really don't need to track the
        // names.
        log::info!("Cleaning up ZFS {}", self.base);
        Command::new(ZFS)
            .args(&["destroy", "-r", &self.base])
            .checked_noio()?;

        Ok(())
    }

    pub fn get_mount(&self) -> String {
        format!("/{}", self.base)
    }

    pub fn get_volume(&self) -> String {
        self.base.to_string()
    }
}
