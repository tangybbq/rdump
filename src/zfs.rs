// SPDX-License-Identifier: Apache-2.0
//! ZFS operations

// For now.
#![allow(unused)]

use anyhow::{anyhow, Result};
use chrono::{Datelike, Local, Timelike};
use regex::{self, Regex};
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::{self, BufRead, BufReader},
    os::unix::io::{AsRawFd, FromRawFd},
    process::{Command, Stdio},
};

use crate::checked::CheckedExt;

// This is an assumption, which seems to be true on at least Fedora and
// Gentoo installs of ZFS.
static ZFS: &'static str = "/sbin/zfs";

#[derive(Debug)]
pub struct Zfs {
    /// The snapshot prefix.  Different prefixes can be used at different times, which will result
    /// in independent snapshots.
    pub prefix: String,
    /// The filesystems found on the system.
    pub filesystems: Vec<Filesystem>,
    /// A re to match snapshot names.
    snap_re: Regex,
    /// The host this involves.
    host: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Filesystem {
    pub name: String,
    pub snaps: Vec<String>,
    pub mount: String,
}

impl Zfs {
    /// Construct a new Zfs retrieving all of the filesystems that are found on this system.
    pub fn new(host: Option<&str>, prefix: &str) -> Result<Zfs> {
        let quoted = regex::escape(prefix);
        let pat = format!("^{}(\\d{{4}})-([-\\d]+)$", quoted);
        let re = Regex::new(&pat)?;

        // Ask ZFS what all of the Filesystems are that it knows about.  Just get the names and
        // mountpoints (which will include all snapshots).  Order of the volumes seems to mostly be
        // lexicographically, at least in some kind of tree order.  The snapshots come out in the
        // order they were created.
        let mut cmd = match host {
            None => Command::new(ZFS),
            Some(host) => {
                let mut cmd = Command::new("ssh");
                cmd.args(&[host, "sudo", ZFS]);
                cmd
            }
        };
        let out = cmd
            .args(&["list", "-H", "-t", "all", "-o", "name,mountpoint"])
            .stderr(Stdio::inherit())
            .checked_output()?;
        let buf = out.stdout;

        let mut builder = SnapBuilder::new();

        for line in BufReader::new(&buf[..]).lines() {
            let line = line?;
            let fields: Vec<_> = line.splitn(2, '\t').collect();
            if fields.len() != 2 {
                return Err(anyhow!("zfs line doesn't have two fields: {:?}", line));
            }
            // fields[0] is now the volume/snap name, and fields[1] is the mountpoint.
            let vols: Vec<_> = fields[0].splitn(2, '@').collect();
            match vols.len() {
                1 => builder.push_volume(vols[0], fields[1]),
                2 => builder.push_snap(vols[0], vols[1]),
                _ => panic!("Unexpected zfs output"),
            }
        }
        let result = builder.into_sets();

        Ok(Zfs {
            prefix: prefix.to_string(),
            filesystems: result,
            snap_re: re,
            host: host.map(|x| x.to_owned()),
        })
    }

    /// Determine the next snapshot number to use, under a given prefix.  The prefix should be a
    /// filesystem name (possibly top level) without a trailing slash.  All filesystems at this
    /// point and under will be considered when looking for volumes.
    pub fn next_under(&self, under: &str) -> Result<usize> {
        let mut next = 0;

        for fs in self.filtered(under)? {
            for snap in &fs.snaps {
                if let Some(caps) = self.snap_re.captures(snap) {
                    let num = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
                    if num + 1 > next {
                        next = num + 1;
                    }
                }
            }
        }

        Ok(next)
    }

    /// Given a snapshot name, return the number of that snapshot, if it matches the pattern,
    /// otherwise None.
    fn snap_number(&self, text: &str) -> Option<usize> {
        self.snap_re
            .captures(text)
            .map(|caps| caps.get(1).unwrap().as_str().parse::<usize>().unwrap())
    }

    /// Return the filtered subset of the filesystems under a given prefix.  Collected into a
    /// vector for type simplicity.
    fn filtered<'a>(&'a self, under: &str) -> Result<Vec<&'a Filesystem>> {
        let re = Regex::new(&format!("^{}(/.*)?$", regex::escape(under)))?;

        Ok(self
            .filesystems
            .iter()
            .filter(|x| re.is_match(&x.name))
            .collect())
    }

    /// Generate a snapshot name of the given index, and the current time.
    pub fn snap_name(&self, index: usize) -> String {
        let now = Local::now();
        let name = format!(
            "{}{:04}-{:04}{:02}{:02}{:02}{:02}",
            self.prefix,
            index,
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute()
        );
        name
    }

    /// Make a new snapshot of the given index on the given filesystem name.  The snapshot itself
    /// will be made recursively.
    pub fn take_snapshot(&self, fs: &str, index: usize) -> Result<()> {
        if self.host.is_some() {
            return Err(anyhow!("Only local snapshots supported"));
        }
        let name = format!("{}@{}", fs, self.snap_name(index));
        println!("Make snapshot: {}", name);
        Command::new(ZFS)
            .args(&["snapshot", "-r", &name])
            .stderr(Stdio::inherit())
            .checked_run()?;
        Ok(())
    }

    /// Make a new snapshot, of a given name.
    pub fn take_named_snapshot(&self, fs: &str, name: &str) -> Result<()> {
        if self.host.is_some() {
            return Err(anyhow!("Only local snapshots supported"));
        }
        let name = format!("{}@{}", fs, name);
        Command::new(ZFS)
            .args(&["snapshot", &name])
            .stderr(Stdio::inherit())
            .checked_run()?;
        Ok(())
    }

    /// Clone one volume tree to another.  Perform should be set to true to
    /// actually do the clones, otherwise it just prints what it would do.
    pub fn clone(
        &self,
        source: &str,
        dest: &str,
        dest_zfs: &Zfs,
        perform: bool,
        excludes: &[&str],
    ) -> Result<()> {
        let excludes = Exclusions::new(excludes)?;

        // Get filtered views of the source and destination filesystems under the given trees.
        let source_fs = self.filtered(source)?;
        let dest_fs = dest_zfs.filtered(dest)?;

        // Make a mapping between the suffixes of the names (including the empty string for one
        // that exactly matches `dest`.  This should be safe as long as `.filtered()` above
        // always returns ones with this string as a prefix.
        let dest_map: HashMap<&str, &Filesystem> = dest_fs
            .iter()
            .map(|&d| (&d.name[dest.len()..], d))
            .collect();

        for src in &source_fs {
            if excludes.is_excluded(&src.name) {
                // println!("Skip: {:?}", src.name);
                continue;
            }

            // Don't clone bookmarks.
            if src.name.contains('#') {
                continue;
            }

            match dest_map.get(&src.name[source.len()..]) {
                Some(d) => {
                    println!("Clone existing: {:?} to {:?}", src.name, d.name);
                    self.clone_one(src, d, dest_zfs, perform)?;
                    if !perform {
                        println!("Clone from:");
                        serde_yaml::to_writer(io::stdout().lock(), src)?;
                        println!("");
                        println!("Clone to:");
                        serde_yaml::to_writer(io::stdout().lock(), d)?;
                        println!("");
                    }
                }
                None => {
                    println!(
                        "Clone fresh: {:?} {:?}+{:?}",
                        src.name,
                        dest,
                        &src.name[source.len()..]
                    );

                    // Construct the new volume.
                    let destfs = Filesystem {
                        name: format!("{}{}", dest, &src.name[source.len()..]),
                        snaps: vec![],
                        mount: "*INVALID*".into(),
                    };

                    if perform {
                        self.make_volume(src, &destfs)?;
                    }
                    self.clone_one(src, &destfs, dest_zfs, perform)?;
                    if !perform {
                        println!("Clone from:");
                        serde_yaml::to_writer(io::stdout().lock(), src)?;
                        println!("");
                        println!("Clone to:");
                        serde_yaml::to_writer(io::stdout().lock(), &destfs)?;
                        println!("");
                    }
                }
            }
        }

        Ok(())
    }

    /// Clone a single filesystem to an existing volume.  We assume there are no snapshots on the
    /// destination that aren't on the source (otherwise it isn't possible to do the clone).
    fn clone_one(
        &self,
        source: &Filesystem,
        dest: &Filesystem,
        dest_zfs: &Zfs,
        perform: bool,
    ) -> Result<()> {
        if let Some(ssnap) = dest.snaps.last() {
            if !source.snaps.contains(ssnap) {
                return Err(anyhow!("Last dest snapshot not present in source"));
            }
            let dsnap = if let Some(dsnap) = source.snaps.last() {
                dsnap
            } else {
                return Err(anyhow!("Source volume has no snapshots"));
            };

            if dsnap == ssnap {
                println!("Destination is up to date");
                return Ok(());
            }

            println!(
                "Clone from {}@{} to {}@{}",
                source.name, ssnap, dest.name, dsnap
            );

            let size = self.estimate_size(&source.name, Some(ssnap), dsnap)?;
            println!("Estimate: {}", humanize_size(size));

            if perform {
                self.do_clone(
                    &source.name,
                    &dest.name,
                    Some(ssnap),
                    dsnap,
                    &dest_zfs,
                    size,
                )?;
            }

            Ok(())
        } else {
            // When doing a full clone, clone from the first snapshot of the volume, and then do a
            // differential backup from that snapshot.
            let dsnap = if let Some(dsnap) = source.snaps.first() {
                dsnap
            } else {
                return Err(anyhow!("Source volume has no snapshots"));
            };

            println!("Full clone from {}@{} to {}", source.name, dsnap, dest.name);

            let size = self.estimate_size(&source.name, None, dsnap)?;
            println!("Estimate: {}", humanize_size(size));
            self.do_clone(&source.name, &dest.name, None, dsnap, &dest_zfs, size)?;

            // Run the clone on the rest of the image.
            let ssnap = dsnap;
            let dsnap = source.snaps.last().expect("source has first but no last");

            // If there are more snapshots to make, clone the rest.
            if ssnap != dsnap {
                let size = self.estimate_size(&source.name, Some(ssnap), dsnap)?;
                if perform {
                    self.do_clone(
                        &source.name,
                        &dest.name,
                        Some(ssnap),
                        dsnap,
                        &dest_zfs,
                        size,
                    )?;
                }
            }

            Ok(())
        }
    }

    /// Use zfs send to estimate the size of this incremental backup.  If the source snap is none,
    /// operate as a full clone.
    fn estimate_size(&self, source: &str, ssnap: Option<&str>, dsnap: &str) -> Result<usize> {
        let mut cmd = Command::new(ZFS);
        cmd.arg("send");
        cmd.arg("-nP");
        if let Some(ssnap) = ssnap {
            cmd.arg("-I");
            cmd.arg(&format!("@{}", ssnap));
        }
        cmd.arg(&format!("{}@{}", source, dsnap));
        cmd.stderr(Stdio::inherit());
        let out = cmd.checked_output()?;

        let buf = out.stdout;
        for line in BufReader::new(&buf[..]).lines() {
            let line = line?;
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() < 2 {
                return Err(anyhow!(
                    "Invalid line from zfs send size estimate: {:?}",
                    line
                ));
            }
            if fields[0] != "size" {
                continue;
            }

            return Ok(fields[1].parse().unwrap());
        }

        Ok(0)
    }

    /// Perform the actual clone.
    fn do_clone(
        &self,
        source: &str,
        dest: &str,
        ssnap: Option<&str>,
        dsnap: &str,
        dest_zfs: &Zfs,
        size: usize,
    ) -> Result<()> {
        // Construct a pipeline from zfs -> pv -> zfs.  PV is used to monitor the progress.
        let mut cmd = Command::new(ZFS);
        cmd.arg("send");
        if let Some(ssnap) = ssnap {
            cmd.arg("-I");
            cmd.arg(&format!("@{}", ssnap));
        }
        cmd.arg(&format!("{}@{}", source, dsnap));
        cmd.stderr(Stdio::inherit());
        cmd.stdout(Stdio::piped());
        let mut sender = cmd.spawn()?;

        let send_out = sender.stdout.as_ref().expect("Child output").as_raw_fd();

        // The unsafe is because using raw descriptors could make them available after they are
        // closed.  These are being given to a spawn, which will be inherited by a fork, and is
        // safe.
        let mut pv = Command::new("pv")
            .args(&["-s", &size.to_string()])
            .stdin(unsafe { Stdio::from_raw_fd(send_out) })
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let pv_out = pv.stdout.as_ref().expect("PV output").as_raw_fd();

        let mut cmd = match &dest_zfs.host {
            None => Command::new(ZFS),
            Some(host) => {
                let mut cmd = Command::new("ssh");
                cmd.args(&[host, "sudo", ZFS]);
                cmd
            }
        };
        let mut receiver = cmd
            .args(&["receive", "-vF", "-x", "mountpoint", dest])
            .stdin(unsafe { Stdio::from_raw_fd(pv_out) })
            .stderr(Stdio::inherit())
            .spawn()?;

        // pv -s <size>
        // zfs receive -vFu <dest>

        if !sender.wait()?.success() {
            return Err(anyhow!("zfs send error"));
        }
        if !pv.wait()?.success() {
            return Err(anyhow!("pv error"));
        }
        if !receiver.wait()?.success() {
            return Err(anyhow!("zfs receive error"));
        }

        Ok(())
    }

    /// Prune old snapshots.  This is a Hanoi-type pruning model, where we keep the most recent
    /// snapshot that has the same number of bits set in it.  In addition, we keep a certain number
    /// `PRUNE_KEEP` of the most recent snapshots.
    pub fn prune_hanoi(&self, fs_name: &str, really: bool) -> Result<()> {
        let fs = if let Some(fs) = self.filesystems.iter().find(|fs| fs.name == fs_name) {
            fs
        } else {
            return Err(anyhow!("Volume not found in zfs {:?}", fs_name));
        };

        // Get all of the snapshots, oldest first, that match this tag, and pair them up with
        // the decoded number.
        let mut snaps: Vec<_> = fs
            .snaps
            .iter()
            .filter_map(|sn| self.snap_number(sn).map(|num| (sn, num)))
            .collect();
        snaps.reverse();

        let mut pops = BTreeSet::<u32>::new();
        let mut to_prune = vec![];

        for item in snaps.iter().enumerate() {
            // Don't prune the most recent ones.
            let index = item.0;
            if index < PRUNE_KEEP {
                continue;
            }

            let name = (item.1).0;
            let num = (item.1).1;

            let bit_count = num.count_ones();
            if pops.contains(&bit_count) {
                let prune_name = format!("{}@{}", fs_name, name);

                to_prune.push(prune_name);
            }
            pops.insert(bit_count);
        }

        // Now do the actual pruning, starting with the oldest ones.
        to_prune.reverse();

        for prune_name in &to_prune {
            println!(
                "{}prune: {}",
                if really { "" } else { "would " },
                prune_name
            );
            if really {
                Command::new(ZFS)
                    .arg("destroy")
                    .arg(&prune_name)
                    .stderr(Stdio::inherit())
                    .checked_run()?;
            }
        }

        Ok(())
    }

    /// Prune a single snapshot (possibly, based on `really`).  This will
    /// attempt to make a bookmark first.
    pub fn prune(&self, vol: &str, snap: &str, really: bool) -> Result<()> {
        if really {
            // Try creating a bookmark.
            println!("pruning: {:?}@{:?}", vol, snap);
            let status = Command::new(ZFS)
                .arg("bookmark")
                .arg(&format!("{}@{}", vol, snap))
                .arg(&format!("{}#{}", vol, snap))
                .stderr(Stdio::inherit())
                .status()?;
            if !status.success() {
                println!("  error creating bookmark");
            }

            // destroy the snapshot
            Command::new(ZFS)
                .arg("destroy")
                .arg(&format!("{}@{}", vol, snap))
                .stderr(Stdio::inherit())
                .checked_run()?;
        } else {
            println!("would prune {:?}@{:?}", vol, snap);
        }
        Ok(())
    }

    /// Construct a new volume at "dest".  Copies over certain attributes (acltype, xattr, atime,
    /// relatime) that are relevant to the snapshot being correct.
    fn make_volume(&self, src: &Filesystem, dest: &Filesystem) -> Result<()> {
        // Read the attributes from the source volume.
        let out = Command::new(ZFS)
            .args(&["get", "-Hp", "all", &src.name])
            .stderr(Stdio::inherit())
            .checked_output()?;
        let buf = out.stdout;
        let mut props = vec![];
        for line in BufReader::new(&buf[..]).lines() {
            let line = line?;
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 4 {
                return Err(anyhow!("zfs get line doesn't have 4 fields: {:?}", line));
            }
            // 0 - name
            // 1 - property
            // 2 - value
            // 3 - source

            // We care about "local" or "received" properties, which are ones that will be set to a
            // value not present.  But, don't include the 'mountpoint' property, so that the backup
            // won't have things randomly mounted.
            if fields[1] == "mountpoint" {
                continue;
            }
            if fields[3] == "local" || fields[3] == "received" {
                props.push("-o".into());
                props.push(format!("{}={}", fields[1], fields[2]));
            }
        }
        println!("   props: {:?}", props);

        Command::new(ZFS)
            .arg("create")
            .args(&props)
            .arg(&dest.name)
            .stderr(Stdio::inherit())
            .checked_run()?;

        Ok(())
    }

    pub fn find_mount(&self, name: &str) -> Result<String> {
        find_mount(name)
    }
}

/// Find where a volume is mounted.  Since Linux can mount ZFS volumes
/// at non-standard locations (specifically for root), use the system's
/// mount table, instead of ZFS.  This also will correctly return an
/// error if the volume is not mounted.
pub fn find_mount(name: &str) -> Result<String> {
    for line in BufReader::new(File::open("/proc/mounts")?).lines() {
        let line = line?;
        let fields: Vec<_> = line.split(' ').collect();
        if fields.len() < 3 || fields[2] != "zfs" {
            continue;
        }
        if fields[0] == name {
            return Ok(fields[1].to_owned());
        }
    }
    return Err(anyhow!("Not mounted {:?}", name));
}

// Construct a Command appropriate for running a zfs command.  This is
// based on the hostname, and will possibly run the command remotely for a
// remove ZFS.  Remote operation only makes sense for some commands.
// fn build_command(

/// The number of recent ones to keep.
const PRUNE_KEEP: usize = 10;

/// A `SnapBuilder` is used to build up the snapshot view of filesystems.
struct SnapBuilder {
    work: Vec<Filesystem>,
}

impl SnapBuilder {
    fn new() -> SnapBuilder {
        SnapBuilder { work: vec![] }
    }

    fn into_sets(self) -> Vec<Filesystem> {
        self.work
    }

    fn push_volume(&mut self, name: &str, mount: &str) {
        self.work.push(Filesystem {
            name: name.to_owned(),
            snaps: vec![],
            mount: mount.to_owned(),
        });
    }

    fn push_snap(&mut self, name: &str, snap: &str) {
        let pos = self.work.len();
        if pos == 0 {
            panic!("Got snapshot from zfs before volume");
        }
        let set = &mut self.work[pos - 1];
        if name != set.name {
            panic!("Got snapshot from zfs without same volume name");
        }
        set.snaps.push(snap.to_owned());
    }
}

// Exclusions are a set of regular expressions matched against source
// filesystem names.  If any match, then that particular backup is skipped.
// Note that this can cause problems if children are backed up and the
// parents are not.  This won't automatically create the parent on the
// destination.
struct Exclusions(Vec<Regex>);

impl Exclusions {
    fn new(excludes: &[&str]) -> Result<Exclusions> {
        // TODO: Figure out how to do this with collect.
        let mut result = vec![];
        for s in excludes {
            result.push(Regex::new(s)?);
        }
        Ok(Exclusions(result))
    }

    fn is_excluded(&self, text: &str) -> bool {
        for re in &self.0 {
            if re.is_match(text) {
                return true;
            }
        }
        false
    }
}

/// Humanize sizes with base-2 SI-like prefixes.
fn humanize_size(size: usize) -> String {
    // This unit table covers at least 80 bits, so the later ones will never be used.
    static UNITS: &'static [&'static str] = &[
        "B  ", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB",
    ];

    let mut value = size as f64;
    let mut unit = 0;

    while value > 1024.0 {
        value /= 1024.0;
        unit += 1;
    }

    let precision = if value < 10.0 {
        3
    } else if value < 100.0 {
        2
    } else {
        2
    };

    format!("{:6.*}{}", precision, value, UNITS[unit])
}
