//! Filesystem based testing
//!
//! This is a test program that implements some testing for rdump that
//! doesn't really make sense in the multi-threaded environment.  First,
//! the tests need to be run as root, and second, it doesn't really make
//! sense for these to be multi-threaded.

use anyhow::Result;
use rdump::{
    actions::{self, Action},
};
use std::{
    path::Path,
};

mod lvm;

fn main() -> Result<()> {
    if users::get_effective_uid() != 0 {
        return Err(anyhow::anyhow!("fstest needs to be run as root"));
    }

    // Initialze the logger that interacts well with rsure's progress
    // meter.
    rsure::log_init();

    // First test, with ext4
    let mut lvm = lvm::LvmTest::setup("joke", "fstest", lvm::FileSystem::Ext4)?;
    backup_lvm(&lvm)?;
    lvm.checkout("v2.0.0")?;
    backup_lvm(&lvm)?;
    lvm.cleanup()?;

    // Second test, with xfs
    let mut lvm = lvm::LvmTest::setup("joke", "xfstest", lvm::FileSystem::Xfs)?;
    backup_lvm(&lvm)?;
    lvm.checkout("v2.0.0")?;
    backup_lvm(&lvm)?;
    lvm.cleanup()?;

    Ok(())
}

fn backup_lvm(lvm: &lvm::LvmTest) -> Result<()> {
    let mp = lvm.mountpoint("");
    let mut a1 = actions::Stamp::new(&Path::new(&mp).join("snapstamp"))?;

    let mut a2 = actions::LvmSnapshot::new(&lvm.pv, &lvm.prefix,
        &format!("{}_snap", lvm.prefix))?;

    let mut a3 = actions::MountSnap::new(&lvm.device_name("_snap"),
        &lvm.mountpoint("_snap"), lvm.fs == lvm::FileSystem::Xfs)?;

    let new_mount = lvm.mountpoint("_snap");
    let mut a4 = actions::LvmRsure::new(&mp, &new_mount)?;

    a1.perform()?;
    a2.perform()?;
    a3.perform()?;
    a4.perform()?;
    a4.cleanup()?;
    a3.cleanup()?;
    a2.cleanup()?;
    a1.cleanup()?;
    Ok(())
}
