// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::EnforcementError;
use std::fmt::Display;
use std::process::Command;
use tracing::{debug, warn};

/// A single external command invocation (program + args). The `tc` / `iptables`
/// managers emit these; keeping commands as data makes the managers unit-testable
/// without a kernel (assert the emitted commands) and gives one place to execute them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new<P, I, S>(program: P, args: I) -> Self
    where
        P: Into<String>,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        CommandSpec {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// The command as a single shell-like string, for logs and assertions.
    pub fn rendered(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Display for CommandSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rendered())
    }
}

/// Executes [`CommandSpec`]s. Production shells out; tests can assert the emitted
/// specs directly without a runner, or supply their own recording implementation.
pub trait CommandRunner: Send + Sync {
    /// Run `cmd`. With `ignore_failure`, a non-zero exit is logged but not returned as
    /// an error (for idempotent teardown / removal of rules that may not exist). A
    /// spawn failure (e.g. missing binary) is always an error.
    fn execute(&self, cmd: &CommandSpec, ignore_failure: bool) -> Result<(), EnforcementError>;
}

/// The real runner: shells out via [`std::process::Command`].
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn execute(&self, cmd: &CommandSpec, ignore_failure: bool) -> Result<(), EnforcementError> {
        debug!("running: {cmd}");
        let output = Command::new(&cmd.program)
            .args(&cmd.args)
            .output()
            .map_err(|source| EnforcementError::Spawn {
                program: cmd.program.clone(),
                source,
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if ignore_failure {
                warn!("ignoring failure of `{cmd}`: {stderr}");
                return Ok(());
            }
            return Err(EnforcementError::CommandFailed {
                command: cmd.rendered(),
                stderr,
            });
        }
        Ok(())
    }
}
