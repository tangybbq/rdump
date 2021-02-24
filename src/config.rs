// SPDX-License-Identifier: Apache-2.0
//! Configuration.

use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    path::Path,
};

use crate::actions::{self, Runner};

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    config: Config,
    simple: Vec<Simple>,
    lvm: Vec<Lvm>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    borg: String,
}

#[derive(Debug, Deserialize)]
pub struct Simple {
    name: String,
    mount: String,
    actions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Lvm {
    name: String,
    mount: String,
    snap: String,
    vg: String,
    lv: String,
    lv_snap: String,
    fs: String,
    actions: Vec<String>,
}

// These phases provide a convenient way to group all of a given phase
// together.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum Phase {
    Timestamp,
    Snapshot,
    Mount,
    Rsure,
    Borg,
}

impl ConfigFile {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<ConfigFile> {
        Ok(serde_yaml::from_reader(File::open(path)?)?)
    }

    pub fn build_runner(&self, names: &[&str]) -> Result<Runner> {
        let names = NameFilter::new(names);

        let mut runners = BTreeMap::new();
        Self::add_runner(&mut runners, Phase::Timestamp, "Timestamps")?;
        Self::add_runner(&mut runners, Phase::Snapshot, "Snapshots")?;
        Self::add_runner(&mut runners, Phase::Mount, "Mount")?;
        Self::add_runner(&mut runners, Phase::Rsure, "Rsure")?;
        Self::add_runner(&mut runners, Phase::Borg, "Borg")?;

        for simp in &self.simple {
            if !names.contains(&simp.name) {
                break;
            }

            simp.add_actions(&mut runners, self)?;
        }

        for lvm in &self.lvm {
            if !names.contains(&lvm.name) {
                break;
            }

            lvm.add_actions(&mut runners, self)?;
        }

        let mut runner = Runner::new()?;

        for (_, run) in runners.into_iter() {
            runner.append(run);
        }

        runner.push(Box::new(actions::Message::new("Finished, cleaning up")?));

        Ok(runner)
    }

    /// Push a new runner, with a banner message for its name.
    fn add_runner(runners: &mut BTreeMap<Phase, Runner>, phase: Phase, message: &str) -> Result<()> {
        let mut run = Runner::new()?;
        run.push(Box::new(actions::Message::new(message)?));
        runners.insert(phase, run);
        Ok(())
    }
}

impl Simple {
    fn add_actions(&self, runners: &mut BTreeMap<Phase, Runner>, config: &ConfigFile) -> Result<()> {
        let a1 = actions::Stamp::new(
            &Path::new(&self.mount).join("snapstamp"))?;
        runners.get_mut(&Phase::Timestamp).unwrap().push(Box::new(a1));

        let local = Utc::now().format("%Y%m%dT%H%M%S");
        let a4 = actions::SimpleRsure::new(&self.mount, &format!("{}", local))?;
        runners.get_mut(&Phase::Rsure).unwrap().push(Box::new(a4));

        let backup_name = format!("{}-{}", self.name, local);
        let a5 = actions::BorgBackup::new(
            &self.mount,
            &config.config.borg,
            &backup_name)?;
        runners.get_mut(&Phase::Borg).unwrap().push(Box::new(a5));

        Ok(())
    }
}

impl Lvm {
    fn add_actions(&self, runners: &mut BTreeMap<Phase, Runner>, config: &ConfigFile) -> Result<()> {
        let a1 = actions::Stamp::new(
            &Path::new(&self.mount).join("snapstamp"))?;
        runners.get_mut(&Phase::Timestamp).unwrap().push(Box::new(a1));

        let a2 = actions::LvmSnapshot::new(&self.vg, &self.lv, &self.lv_snap)?;
        runners.get_mut(&Phase::Snapshot).unwrap().push(Box::new(a2));

        let snap_device = format!("/dev/{}/{}", self.vg, self.lv_snap);
        let a3 = actions::MountSnap::new(&snap_device, &self.snap,
            self.fs == "xfs")?;
        runners.get_mut(&Phase::Mount).unwrap().push(Box::new(a3));

        let local = Utc::now().format("%Y%m%dT%H%M%S");
        let a4 = actions::LvmRsure::new(&self.mount, &self.snap,
            &format!("{}", local))?;
        runners.get_mut(&Phase::Rsure).unwrap().push(Box::new(a4));

        let backup_name = format!("{}-{}", self.name, local);
        let a5 = actions::BorgBackup::new(
            &self.snap,
            &config.config.borg,
            &backup_name)?;
        runners.get_mut(&Phase::Borg).unwrap().push(Box::new(a5));

        Ok(())
    }
}

struct NameFilter<'a> {
    names: Option<HashSet<&'a str>>,
}

impl<'a> NameFilter<'a> {
    fn new<'b>(names: &[&'b str]) -> NameFilter<'b> {
        if names.len() == 0 {
            NameFilter { names: None }
        } else {
            NameFilter { names: Some(names.iter().cloned().collect()) }
        }
    }

    fn contains(&self, name: &str) -> bool {
        match self.names {
            None => true,
            Some(ref set) => set.contains(name),
        }
    }
}
