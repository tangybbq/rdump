//! An extension to Command to allow checked runs.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::process::{Output, Stdio};
use tokio::process::Command;

#[async_trait]
pub trait CheckedExt {
    /// Run the given command, normalizing to the local Result type, and returning a local error if
    /// the command doesn't return success.
    async fn checked_run(&mut self) -> Result<()>;

    /// Run command, collecting all of its output.  Runs Command's `output` method, with an
    /// additional check of the status result.
    async fn checked_output(&mut self) -> Result<Output>;

    /// Run a command, returning an error if the command doesn't return
    /// success.  Like `checked_run`, but also maps stderr to stdout, and
    /// stdin to null.
    async fn checked_noio(&mut self) -> Result<()>;
}

#[async_trait]
impl CheckedExt for Command {
    async fn checked_run(&mut self) -> Result<()> {
        let status = self.status().await?;
        if !status.success() {
            return Err(anyhow!("Error running command: {:?} ({:?})", self, status));
        }
        Ok(())
    }

    async fn checked_output(&mut self) -> Result<Output> {
        let out = self.output().await?;
        if !out.status.success() {
            return Err(anyhow!("Error running command: {:?} ({:?})", self, out.status));
        }
        Ok(out)
    }

    async fn checked_noio(&mut self) -> Result<()> {
        self.stderr(Stdio::inherit());
        self.stdin(Stdio::null());
        self.checked_run().await?;
        Ok(())
    }
}
