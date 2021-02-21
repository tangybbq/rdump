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

    env_logger::init();

    // First test, with ext4
    let mut lvm = lvm::LvmTest::setup("joke", "fstest", lvm::FileSystem::Ext4)?;

    let mp = lvm.mountpoint("");
    let mut a1 = actions::Stamp::new(&Path::new(&mp).join("snapstamp"))?;
    a1.perform()?;
    a1.cleanup()?;

    lvm.cleanup()?;

    // Second test, with xfs
    let mut lvm = lvm::LvmTest::setup("joke", "xfstest", lvm::FileSystem::Xfs)?;
    lvm.cleanup()?;

    Ok(())
}
