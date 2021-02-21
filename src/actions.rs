//! Backup actions.
//!
//! These actions can be assembled into an action sequence where some may
//! have setup and teardown aspects.  The runner will perform the teardowns
//! even if one of the later actions fail.

use anyhow::Result;
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    process::Command,
};

use crate::checked::CheckedExt;

pub trait Action {
    fn perform(&mut self) -> Result<()>;
    fn cleanup(&mut self) -> Result<()>;
}

/// An action that creates a timestamp in the filesystem of question.  This
/// is used by some backup tools to avoid issues with files that are
/// modified between when a snapshot is created and an incremental backup
/// is performed.
pub struct Stamp {
    path: String,
}

impl Stamp {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Stamp> {
        Ok(Stamp{ path: path.as_ref().to_str().unwrap().into() })
    }
}

impl Action for Stamp {
    fn perform(&mut self) -> Result<()> {
        // Since there isn't a convenient "touch" in std, just write
        // something to the file, which will update the timestamp.
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        writeln!(&mut file, "Backup timestamp")?;

        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        // No cleanup.  We leave the stamp present for possible future
        // backups.
        Ok(())
    }
}

pub struct LvmSnapshot {
    pv: String,
    base: String,
    snap: String,
}

impl LvmSnapshot {
    pub fn new(pv: &str, base: &str, snap: &str) -> Result<LvmSnapshot> {
        Ok(LvmSnapshot {
            pv: pv.into(),
            base: base.into(),
            snap: snap.into(),
        })
    }
}

impl Action for LvmSnapshot {
    fn perform(&mut self) -> Result<()> {
        Command::new("lvcreate")
            .args(&["-L", "1g", "-s", "-n", &self.snap,
                &format!("{}/{}", self.pv, self.base)])
            .checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        Command::new("lvremove")
            .args(&["-f", &format!("{}/{}", self.pv, self.snap)])
            .checked_noio()?;
        Ok(())
    }
}

pub struct MountSnap {
    device: String,
    mount: String,
}

impl MountSnap {
    pub fn new(device: &str, mount: &str) -> Result<MountSnap> {
        Ok(MountSnap {
            device: device.into(),
            mount: mount.into(),
        })
    }
}

impl Action for MountSnap {
    fn perform(&mut self) -> Result<()> {
        Command::new("mkdir")
            .args(&["-p", &self.mount])
            .checked_noio()?;
        Command::new("mount")
            .args(&[&self.device, &self.mount])
            .checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        Command::new("umount")
            .arg(&self.mount)
            .checked_noio()?;
        Ok(())
    }
}
