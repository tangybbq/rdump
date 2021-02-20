//! Filesystem based testing
//!
//! This is a test program that implements some testing for rdump that
//! doesn't really make sense in the multi-threaded environment.  First,
//! the tests need to be run as root, and second, it doesn't really make
//! sense for these to be multi-threaded.

use anyhow::Result;

mod lvm;

#[tokio::main]
async fn main() -> Result<()> {
    if users::get_effective_uid() != 0 {
        return Err(anyhow::anyhow!("fstest needs to be run as root"));
    }

    env_logger::init();

    // First test, with ext4
    let mut lvm = lvm::LvmTest::setup("joke", "fstest", lvm::FileSystem::Ext4).await?;
    lvm.cleanup().await?;

    // Second test, with xfs
    let mut lvm = lvm::LvmTest::setup("joke", "xfstest", lvm::FileSystem::Xfs).await?;
    lvm.cleanup().await?;

    Ok(())
}
