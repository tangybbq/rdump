// SPDX-License-Identifier: Apache-2.0
//! LVM management for the test.
//!
//! To simulate active filesystems, we will use git, and in this case, just
//! checkout a Zephyr tree, checking out various releases to cycle through
//! the changed data.
//!
//! We currently support ext4 and xfs filesystems for the test.
//!
//! Each will be on a given 'pv', with the base filesystem being called
//! 'prefix'.

use rdump::CheckedExt;

use anyhow::Result;
use std::{mem, process::Command};

static ZEPHYR_PARENT: &'static str = "/lint/zephyr/zephyr.git";
static MOUNT_BASE: &'static str = "/mnt/test";

pub struct LvmTest {
    pub pv: String,
    pub prefix: String,
    pub fs: FileSystem,
    volume_created: bool,
    mount: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileSystem {
    Ext4,
    Xfs,
}

impl LvmTest {
    /// Set up a new filesystem on the given pv with the given prefix.
    pub fn setup(pv: &str, prefix: &str, fs: FileSystem) -> Result<LvmTest> {
        // Create a 5GB volume to house this data.
        // "--yes" is somewhat dangerous but there doesn't seem to be any
        // way to get lvcreate to wipte the signatures without it becoming
        // interactive.
        log::info!("Creating lvm volume {}/{}", pv, prefix);
        Command::new("lvcreate")
            .args(&["-L", "5G", "--yes", "-n", prefix, pv])
            .checked_noio()?;

        let mut result = LvmTest {
            pv: pv.to_owned(),
            prefix: prefix.to_owned(),
            fs,
            volume_created: true,
            mount: None,
        };

        result.mkfs()?;
        result.mount("")?;

        let mp = result.mountpoint("");

        // Clone a zephyr tree, and set to the first version.
        let dest = format!("{}/zephyr", mp);
        log::info!("Cloning git repo into fs");
        Command::new("git")
            .args(&["clone", ZEPHYR_PARENT, &dest])
            .checked_noio()?;
        Command::new("git")
            .args(&["checkout", "-q", "v1.0.0"])
            .current_dir(&dest)
            .checked_noio()?;

        log::info!("Filesystem mounted at {}", mp);
        Ok(result)
    }

    /// Update the checkedout tree to a specific version, this simulates
    /// changes in the filesystem to be backed up.
    pub fn checkout(&self, version: &str) -> Result<()> {
        let mp = self.mountpoint("");

        let dest = format!("{}/zephyr", mp);
        log::info!("Moving FS data in {} to {}", dest, version);
        Command::new("git")
            .args(&["checkout", "-q", version])
            .current_dir(&dest)
            .checked_noio()?;
        Ok(())
    }

    fn mkfs(&self) -> Result<()> {
        let device = self.device_name("");

        match self.fs {
            FileSystem::Ext4 => {
                Command::new("mkfs.ext4").arg(&device).checked_noio()?;
            }
            FileSystem::Xfs => {
                Command::new("mkfs.xfs").arg(&device).checked_noio()?;
            }
        }

        Ok(())
    }

    /// Mount this filesystem/prefix.
    fn mount(&mut self, extra: &str) -> Result<()> {
        let mp = self.mountpoint(extra);

        // Make sure the mount directory exists.
        Command::new("mkdir").args(&["-p", &mp]).checked_noio()?;

        match self.fs {
            FileSystem::Ext4 => {
                Command::new("mount")
                    .args(&[&self.device_name(extra), &mp])
                    .checked_noio()?;
            }
            FileSystem::Xfs => {
                Command::new("mount")
                    .args(&[&self.device_name(extra), &mp])
                    .checked_noio()?;
            }
        }

        // If the mount works, stick the mountpoint so we can know to
        // unmount it.
        self.mount = Some(mp);
        Ok(())
    }

    /// Return the device name for this filesystem, with a possible extra
    /// appended.
    pub fn device_name(&self, extra: &str) -> String {
        format!("/dev/{}/{}{}", self.pv, self.prefix, extra)
    }

    /// Return a mountpoint for this filesystem, with a possible extra
    /// appended.
    pub fn mountpoint(&self, extra: &str) -> String {
        format!("{}/{}{}", MOUNT_BASE, self.prefix, extra)
    }

    /// Cleanup.  TODO: Use drop for this.
    pub fn cleanup(&mut self) -> Result<()> {
        log::info!("Lvm cleanup");
        if let Some(mp) = self.mount.take() {
            log::info!("Unmounting {}", mp);
            Command::new("umount").arg(&mp).checked_noio()?;
        }

        if mem::replace(&mut self.volume_created, false) {
            log::info!("Destroying LVM {}/{}", self.pv, self.prefix);
            Command::new("lvremove")
                .args(&["-f", &format!("{}/{}", self.pv, self.prefix)])
                .checked_noio()?;
        }
        log::info!("Lvm cleanup done");

        Ok(())
    }
}
