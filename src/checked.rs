// SPDX-License-Identifier: Apache-2.0
//! An extension to Command to allow checked runs.

use anyhow::{anyhow, Result};
use std::process::{Command, Output, Stdio};

pub trait CheckedExt {
    /// Run the given command, normalizing to the local Result type, and returning a local error if
    /// the command doesn't return success.
    fn checked_run(&mut self) -> Result<()>;

    /// Run command, collecting all of its output.  Runs Command's `output` method, with an
    /// additional check of the status result.
    fn checked_output(&mut self) -> Result<Output>;

    /// Run a command, returning an error if the command doesn't return
    /// success.  Like `checked_run`, but also maps stderr to stdout, and
    /// stdin to null.
    fn checked_noio(&mut self) -> Result<()>;
}

impl CheckedExt for Command {
    fn checked_run(&mut self) -> Result<()> {
        let status = self.status()?;
        if !status.success() {
            return Err(anyhow!("Error running command: {:?} ({:?})", self, status));
        }
        Ok(())
    }

    fn checked_output(&mut self) -> Result<Output> {
        let out = self.output()?;
        if !out.status.success() {
            return Err(anyhow!(
                "Error running command: {:?} ({:?})",
                self,
                out.status
            ));
        }
        Ok(out)
    }

    fn checked_noio(&mut self) -> Result<()> {
        self.stderr(Stdio::inherit());
        self.stdin(Stdio::null());
        self.checked_run()?;
        Ok(())
    }
}
