//! Sudo support
//!
//! Many of the rdump actions need to be performed as root.  Rather than
//! run everything as root, we can optionally use sudo to elevate
//! priveleges.  This should be selectable via configuration
//!
//! In order to keep sudo from timing out, we will create an async task
//! that periodically wakes up and just runs a simple sudo command.  If
//! sudo is not selected (presuming we're already running as root), this
//! will not be started, and commands will just be run directly.

use crate::Result;
use anyhow::anyhow;
use tokio::{
    process::Command,
};
use tokio::time;

pub struct Sudo {
    // The join handle for the background task, so that we can kill it when
    // the last Sudo goes out of scope.  The challenge here is that the
    // background task itself can't hold a reference to it.
    child: Option<tokio::task::JoinHandle<()>>,
}

impl Sudo {
    /// Possibly start a sudo runner.  This will determine if sudo is
    /// needed based both on the `enable` flag (presumably from a config
    /// file) as well as by determining if we are already running as root.
    pub async fn start(enable: bool) -> Result<Sudo> {
        let is_root = users::get_effective_uid() == 0;

        let enabled = enable && !is_root;

        if enabled {
            Sudo::poke_sudo().await?
        }

        let child = if enabled {
            Some(tokio::spawn(async {
                let interval = time::interval(time::Duration::from_secs(60));
                tokio::pin!(interval);

                // Skip the first, as it happens immediately.
                interval.as_mut().tick().await;
                loop {
                    interval.as_mut().tick().await;
                    match Sudo::poke_sudo().await {
                        Ok(_) => (),
                        Err(e) => {
                            log::error!("Error running background sudo: {:?}", e);
                            break;
                        }
                    }
                }
            }))
        } else {
            None
        };

        Ok(Sudo { child })
    }

    /// Return a new Command to run in this context, but with the proper
    /// wrapper to perform as root.
    pub fn new_cmd(&self, cmd: &str) -> Command {
        if self.child.is_some() {
            let mut res = Command::new("sudo");
            res.arg(cmd);
            res
        } else {
            Command::new(cmd)
        }
    }

    /// Ensure that sudo has been run.  This may prompt for a password the
    /// first time, but as long as it is run regularly, should keep
    /// additional prompts from being needed.
    async fn poke_sudo() -> Result<()> {
        let status = Command::new("sudo")
            .arg("true")
            .status().await?;
        if !status.success() {
            return Err(anyhow!("unable to run sudo: {:?}", status.code()));
        }

        Ok(())
    }
}

// Drop for Sudo will stop the background task from running.
impl Drop for Sudo {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            log::info!("Stopping Sudo");
            child.abort();
        }
    }
}
