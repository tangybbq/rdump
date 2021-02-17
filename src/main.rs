//! Backup driver

use anyhow::Result;
use clap::{
    App,
    load_yaml,
};
use std::{
    path::Path,
};
use rdump::Zfs;

#[tokio::main]
async fn main() -> Result<()> {
    if false {
        wasy().await?;
    }

    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    println!("{:#?}", matches);

    let config = matches.value_of("config").unwrap_or("rdump.yaml");
    println!("config: {:?}", config);

    if let Some(matches) = matches.subcommand_matches("clone") {
        let volume = matches.value_of("VOLUME").unwrap();
        println!("volume: {:?}", volume);
    }

    Ok(())
}

async fn wasy() -> Result<()> {
    let fs = Zfs::new(None, "da2021").await?;
    // println!("{:#?}", fs);

    // Scan each filesystem that has a mountpoint, looking for a
    // .zfssync.yml file.
    let mut valid = vec![];
    for fs in &fs.filesystems {
        let base = Path::new(&fs.mount);
        let work = base.join(".zfssync.yml");
        match tokio::fs::metadata(&work).await {
            Ok(_) => valid.push(fs),
            Err(_) => (),
        }
    }
    println!("Valid: {:?}", valid);
    Ok(())
}
