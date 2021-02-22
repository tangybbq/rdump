//! Actions runner.
//!
//! Our goal here is to be able to register a series of actions to perform
//! a backup.  Each of these actions has a possible cleanup.  We want to
//! run the cleanup on all actions that have completed, regardless of any
//! errors that may have happened.

use anyhow::Result;
use super::Action;

pub struct Runner {
    actions: Vec<Box<dyn Action>>,
}

impl Runner {
    pub fn new() -> Result<Runner> {
        Ok(Runner {
            actions: Vec::new(),
        })
    }

    /// Add a new action, to be performed after previously added actions.
    pub fn push(&mut self, action: Box<dyn Action>) {
        self.actions.push(action);
    }

    /// Perform all of the actions, and any appropriate cleanup.  Note that
    /// this consumes self, and all actions registered will be dropped.
    /// If any perform results in an Error, that will be the return result
    /// of this function, although cleanups will be called for other
    /// actions.
    pub fn run(self, pretend: bool) -> Result<()> {
        let mut cleanups = vec![];

        for mut action in self.actions.into_iter() {
            if pretend {
                println!("would: {}", action.describe());
            } else {
                // TODO: Add a descriptive method.
                match action.perform() {
                    Ok(()) => cleanups.push(action),
                    Err(err) => {
                        log::error!("Error with action: {:?}", err);
                        Self::run_cleanups(cleanups);
                        return Err(err);
                    },
                }
            }
        }

        Ok(())
    }

    /// Perform all of the given cleanups, in reverse order.  Errors are
    /// logged, but don't otherwise stop the rest of the cleanups from
    /// running.
    fn run_cleanups(mut cleanups: Vec<Box<dyn Action>>) {
        while let Some(mut action) = cleanups.pop() {
            // TODO: Add descriptive method.
            match action.cleanup() {
                Ok(()) => (),
                Err(err) => log::error!("Cleanup error: {:?}", err),
            }
        }
    }

    /// Consume the argument, appending all actions from it into the self
    /// runner.
    pub fn append(&mut self, mut other: Runner) {
        self.actions.append(&mut other.actions);
    }
}
