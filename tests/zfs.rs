//! Test the zfs module.

use rdump::Zfs;
use async_once::AsyncOnce;
use lazy_static::lazy_static;
use rdump::{
    Sudo,
};

// We'll use the once to initialize the logging system, but since
// lazy_static items are never destroyed, it would leave background sudo
// tasks running.
lazy_static! {
    static ref TEST_INIT: AsyncOnce<()> = AsyncOnce::new(async {
        env_logger::init();
        log::info!("TEST_INIT starting");
    });
}

// Instead, for initialization, we'll let each task have its own Sudo. This
// will result in some extra runners, but they will be cleaned up when each
// test is unwound.
async fn init() -> Sudo {
    TEST_INIT.get().await;
    Sudo::start(true).await.unwrap()
}

#[tokio::test]
async fn setup_example() {
    let _sudo = init().await;
    log::info!("setup example");

    let info = Zfs::new(None, "uniquetest").await.unwrap();
    log::info!("ZFS found {} volumes", info.filesystems.len());
}

#[tokio::test]
async fn second_example() {
    let _sudo = init().await;
    log::info!("second example");
}
