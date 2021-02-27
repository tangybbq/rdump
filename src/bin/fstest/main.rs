// SPDX-License-Identifier: Apache-2.0
//! Filesystem based testing
//!
//! This is a test program that implements some testing for rdump that
//! doesn't really make sense in the multi-threaded environment.  First,
//! the tests need to be run as root, and second, it doesn't really make
//! sense for these to be multi-threaded.

use anyhow::Result;
use chrono::Utc;
use rdump::actions::{self, Runner};
use std::path::Path;

mod lvm;
mod zfs;

fn main() -> Result<()> {
    if users::get_effective_uid() != 0 {
        return Err(anyhow::anyhow!("fstest needs to be run as root"));
    }

    // Initialze the logger that interacts well with rsure's progress
    // meter.
    rsure::log_init();

    // First test, with ext4
    let mut lvm = lvm::LvmTest::setup("joke", "fstest", lvm::FileSystem::Ext4)?;
    let zfs = zfs::ZfsTest::setup()?;
    backup_lvm(&lvm, &zfs)?;
    lvm.checkout("v2.0.0")?;
    backup_lvm(&lvm, &zfs)?;
    zfs.cleanup()?;
    lvm.cleanup()?;

    // Second test, with xfs
    let mut lvm = lvm::LvmTest::setup("joke", "xfstest", lvm::FileSystem::Xfs)?;
    let zfs = zfs::ZfsTest::setup()?;
    backup_lvm(&lvm, &zfs)?;
    lvm.checkout("v2.0.0")?;
    backup_lvm(&lvm, &zfs)?;
    zfs.cleanup()?;
    lvm.cleanup()?;

    Ok(())
}

fn backup_lvm(lvm: &lvm::LvmTest, zfs: &zfs::ZfsTest) -> Result<()> {
    let mut run = Runner::new()?;

    let mp = lvm.mountpoint("");
    run.push(Box::new(actions::Stamp::new(
        &Path::new(&mp).join("snapstamp"),
    )?));

    run.push(Box::new(actions::LvmSnapshot::new(
        &lvm.pv,
        &lvm.prefix,
        &format!("{}_snap", lvm.prefix),
    )?));

    run.push(Box::new(actions::MountSnap::new(
        &lvm.device_name("_snap"),
        &lvm.mountpoint("_snap"),
        lvm.fs == lvm::FileSystem::Xfs,
    )?));

    let local = Utc::now().format("%Y%m%dT%H%M%S");
    let new_mount = lvm.mountpoint("_snap");
    run.push(Box::new(actions::LvmRsure::new(
        &mp,
        &new_mount,
        &format!("{}", local),
    )?));

    let backup_name = format!("{}-{}", lvm.prefix, local);
    run.push(Box::new(actions::BorgBackup::new(
        &new_mount,
        "/home/davidb/back/fstest-borg.sh",
        &backup_name,
    )?));

    run.push(Box::new(actions::Rsync::new(
        &new_mount,
        &zfs.get_mount(),
        true,
        true,
    )?));

    run.push(Box::new(actions::ZfsSnapshot::new(
        &zfs.get_volume(),
        &format!("{}", local),
    )?));

    run.run(false)?;

    Ok(())
}
