//! Backup driver

use anyhow::Result;
use clap::{
    App,
    load_yaml,
};
use std::{
    fs,
    path::Path,
    thread,
};
use rdump::Zfs;

fn main() -> Result<()> {
    if false {
        wasy()?;
    }

    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    println!("{:#?}", matches);

    let config = matches.value_of("config").unwrap_or("rdump.yaml");
    println!("config: {:?}", config);

    if let Some(matches) = matches.subcommand_matches("clone") {
        let volume = matches.value_of("VOLUME").unwrap();
        let _sudo = rdump::Sudo::start(true)?;
        println!("volume: {:?}", volume);
        println!("Sleeping 1 minute");
        thread::sleep(std::time::Duration::from_secs(60));
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
