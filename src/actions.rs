//! Backup actions.
//!
//! These actions can be assembled into an action sequence where some may
//! have setup and teardown aspects.  The runner will perform the teardowns
//! even if one of the later actions fail.

use anyhow::{anyhow, Result};
use log::info;
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
        info!("Writing backup stamp: {:?}", self.path);
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
        info!("LVM2 snapshot of {}/{} to {}", self.pv, self.base, self.snap);
        Command::new("lvcreate")
            .args(&["-L", "1g", "-s", "-n", &self.snap,
                &format!("{}/{}", self.pv, self.base)])
            .checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        info!("Cleanup lvm snapshot {}/{}", self.pv, self.snap);
        Command::new("lvremove")
            .args(&["-f", &format!("{}/{}", self.pv, self.snap)])
            .checked_noio()?;
        Ok(())
    }
}

pub struct MountSnap {
    device: String,
    mount: String,
    is_xfs: bool,
}

impl MountSnap {
    pub fn new(device: &str, mount: &str, is_xfs: bool) -> Result<MountSnap> {
        Ok(MountSnap {
            device: device.into(),
            mount: mount.into(),
            is_xfs,
        })
    }
}

impl Action for MountSnap {
    fn perform(&mut self) -> Result<()> {
        info!("Mount LVM2 snapshot {} to {}", self.device, self.mount);
        Command::new("mkdir")
            .args(&["-p", &self.mount])
            .checked_noio()?;
        let opt = if self.is_xfs {
            "nouuid,noatime"
        } else {
            "noatime"
        };
        Command::new("mount")
            .args(&[&self.device, "-o", opt, &self.mount])
            .checked_noio()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        info!("Unmount lvm2 snapshot at {}", self.mount);
        Command::new("umount")
            .arg(&self.mount)
            .checked_noio()?;
        Ok(())
    }
}

pub struct LvmRsure {
    base_mount: String,
    mount: String,
}

impl LvmRsure {
    pub fn new(base_mount: &str, mount: &str) -> Result<LvmRsure> {
        Ok(LvmRsure {
            base_mount: base_mount.into(),
            mount: mount.into(),
        })
    }
}

// Big TODO: Need to make the error type in rsure a real error type.
impl Action for LvmRsure {
    fn perform(&mut self) -> Result<()> {
        let surefile = format!("{}/2sure.dat.gz", self.mount);
        let is_update = Path::new(&surefile).is_file();

        info!("Rsure scan of {} to {}", self.mount, surefile);
        let store = match rsure::parse_store(&surefile) {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("Error parsing store: {:?}", e)),
        };

        let mut tags = rsure::StoreTags::new();
        tags.insert("name".into(), "TODO: put name here".into());

        match rsure::update(&self.mount, &*store, is_update, &tags) {
            Ok(()) => (),
            Err(e) => return Err(anyhow!("Error running rsure update: {:?}", e)),
        }

        info!("Copy rsure file {} to {}", surefile, self.base_mount);
        // Use cp command for -p to preserve as much as possible.
        Command::new("cp")
            .args(&["-p", &surefile, &self.base_mount])
            .checked_noio()?;

        Ok(())
    }
    fn cleanup(&mut self) -> Result<()> {
        // No cleanup
        Ok(())
    }
}
