//! LVM setup for the test environment.
//!
//! To simulate active filesystems, we will use git, and in this case, just
//! checkout a Zephyr tree, and checkout various releases to cycle through
//! the changed data.
//!
//! We currently support ext4 and xfs filesystems for the test.
//!
//! Each of these will be on a given 'pv', with the base filesystem being
//! called 'prefix'.

use crate::init;
use rdump::{
    CheckedExt,
    Sudo,
};

use anyhow::Result;
use std::process::Stdio;

// Constants to be put in config at some point.
static ZEPHYR_PARENT: &'static str = "/lint/zephyr/zephyr.git";
static TEST_PV: &'static str = "joke";
static MOUNT_BASE: &'static str = "/mnt/test";

pub struct LvmTest {
    pv: String,
    prefix: String,
    fs: FileSystem,
}

#[derive(Copy, Clone, Debug)]
pub enum FileSystem {
    Ext4,
    Xfs,
}

impl LvmTest {
    /// Set up a new filesystem on the given pv with the given prefix.
    pub async fn setup(sudo: &Sudo, pv: &str, prefix: &str, fs: FileSystem) -> Result<LvmTest> {
        // Create a 5GB volume to house this data.
        // "--yes" is somewhat dangerous but there doesn't seem to be any
        // way to get lvcreate to wipe the signatures without it becoming
        // interactive.
        sudo.new_cmd("lvcreate")
            .args(&["-L", "5G", "--yes", "-n", prefix, pv])
            .stderr(Stdio::inherit())
            .checked_run().await?;

        let result = LvmTest {
            pv: pv.to_owned(),
            prefix: prefix.to_owned(),
            fs,
        };

        // Create the appropriate filesystem.
        fs.mkfs(sudo, &result).await?;

        // Mount the filesystem.
        let mp = result.mountpoint("");
        // tokio::fs::create_dir_all(&mp).await?;
        sudo.new_cmd("mkdir")
            .args(&["-p", &mp])
            .stderr(Stdio::inherit())
            .checked_run().await?;

        sudo.new_cmd("mount")
            .args(&[&result.device_name(""), &result.mountpoint("")])
            .stderr(Stdio::inherit())
            .checked_run().await?;

        // Clone the zephyr tree there.
        let dest = format!("{}/zephyr", mp);
        sudo.new_cmd("git")
            .args(&["clone", ZEPHYR_PARENT, &dest])
            .stderr(Stdio::inherit())
            .checked_run().await?;

        Ok(result)
    }

    /// Return the device name for this filesystem, with a possible extra
    /// appended.
    pub fn device_name(&self, extra: &str) -> String {
        format!("/dev/{}/{}{}", self.pv, self.prefix, extra)
    }

    /// Return a mountpoint for this filesystem, with a possible extra
    /// appended (which will just be made for it).
    pub fn mountpoint(&self, extra: &str) -> String {
        format!("{}/{}{}", MOUNT_BASE, self.prefix, extra)
    }
}

impl FileSystem {
    async fn mkfs(self, sudo: &Sudo, lvm: &LvmTest) -> Result<()> {
        let device = lvm.device_name("");

        match self {
            FileSystem::Ext4 => {
                sudo.new_cmd("mkfs.ext4")
                    .arg(&device)
                    .stderr(Stdio::inherit())
                    .checked_run().await?;
            }
            FileSystem::Xfs => {
                unimplemented!()
            }
        }

        Ok(())
    }
}

#[tokio::test]
async fn setup_ext4() {
    let sudo = init().await;
    let _lt = LvmTest::setup(&sudo, TEST_PV, "stest-aaa", FileSystem::Ext4).await.unwrap();
}
