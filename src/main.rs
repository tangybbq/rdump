//! Backup driver

use anyhow::Result;
use zfs::Zfs;

mod checked;
mod zfs;

fn main() -> Result<()> {
    let fs = Zfs::new(None, "lint")?;
    println!("{:#?}", fs);
    Ok(())
}
