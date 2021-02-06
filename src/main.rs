//! Backup driver

use anyhow::Result;
use zfs::Zfs;

mod checked;
mod zfs;

#[tokio::main]
async fn main() -> Result<()> {
    let fs = Zfs::new(None, "lint").await?;
    println!("{:#?}", fs);
    Ok(())
}
