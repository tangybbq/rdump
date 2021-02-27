// SPDX-License-Identifier: Apache-2.0
//! Backup driver

use anyhow::Result;
use clap::{load_yaml, App};
use rdump::{ConfigFile, Zfs};
use std::{fs, path::Path, thread};

fn main() -> Result<()> {
    if false {
        wasy()?;
    }

    rsure::log_init();

    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    // println!("{:#?}", matches);

    let cname = matches.value_of("config").unwrap_or("rdump.yaml");
    // println!("cname: {:?}", cname);

    let config = ConfigFile::load(&cname)?;
    // println!("Config: {:#?}", config);

    if let Some(matches) = matches.subcommand_matches("clone") {
        let volume = matches.value_of("VOLUME").unwrap();
        let _sudo = rdump::Sudo::start(true)?;
        println!("volume: {:?}", volume);
        println!("Sleeping 1 minute");
        thread::sleep(std::time::Duration::from_secs(60));
    } else if let Some(matches) = matches.subcommand_matches("backup") {
        let pretend = matches.occurrences_of("pretend") > 0;

        let names: Vec<_> = matches
            .values_of("NAME")
            .map(|c| c.collect())
            .unwrap_or(vec![]);

        let runner = config.build_runner(&names)?;
        runner.run(pretend)?;
    }

    Ok(())
}

fn wasy() -> Result<()> {
    let fs = Zfs::new(None, "da2021")?;
    // println!("{:#?}", fs);

    // Scan each filesystem that has a mountpoint, looking for a
    // .zfssync.yml file.
    let mut valid = vec![];
    for fs in &fs.filesystems {
        let base = Path::new(&fs.mount);
        let work = base.join(".zfssync.yml");
        match fs::metadata(&work) {
            Ok(_) => valid.push(fs),
            Err(_) => (),
        }
    }
    println!("Valid: {:?}", valid);
    Ok(())
}
