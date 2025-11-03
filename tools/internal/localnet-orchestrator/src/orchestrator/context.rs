// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{async_with_progress, exec_cmd_with_output, exec_fallible_cmd_with_output};
use console::{Emoji, style};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use nym_validator_client::nyxd::Coin;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::IsTerminal;
use std::process::{ExitStatus, Output};
use std::time::Instant;
use tracing::info;

#[derive(Default)]
pub(crate) struct Empty;

pub(crate) struct LocalnetContext<T = Empty> {
    pub(crate) data: T,
    progress_tracker: ProgressTracker,
    current_step: usize,
    steps: usize,
}

pub(crate) fn ephemeral_context(msg: impl AsRef<str>) -> LocalnetContext<Empty> {
    LocalnetContext::ephemeral(msg)
}

impl LocalnetContext<Empty> {
    pub(crate) fn ephemeral(msg: impl AsRef<str>) -> Self {
        LocalnetContext::new(Empty, 1, msg)
    }
}

impl<T> LocalnetContext<T> {
    pub(crate) fn new(data: T, steps: usize, msg: impl AsRef<str>) -> Self {
        LocalnetContext {
            data,
            progress_tracker: ProgressTracker::new(msg),
            current_step: 0,
            steps,
        }
    }

    pub(crate) fn skip_steps(&mut self, steps: usize) {
        self.current_step += steps;
    }

    pub(crate) fn begin_next_step(
        &mut self,
        msg: impl AsRef<str>,
        emoji: impl Into<Option<&'static str>>,
    ) {
        self.current_step += 1;

        let emoji = match emoji.into() {
            Some(emoji) => Emoji::new(emoji, ">"),
            None => Emoji(">", ">"),
        };
        let msg = msg.as_ref();

        let progress = format!("{}/{}", self.current_step, self.steps);
        self.println(format!("{emoji} {} {msg}", style(progress).bold().dim()));
        self.set_pb_prefix("");
        self.set_pb_message(format!("{emoji} {msg}"))
    }

    pub(crate) fn println<I: AsRef<str>>(&self, msg: I) {
        self.progress_tracker.println(msg)
    }

    pub(crate) fn println_with_emoji<I: AsRef<str>>(&self, msg: I, emoji: &str) {
        self.progress_tracker.println_with_emoji(msg, emoji)
    }

    pub(crate) fn set_pb_prefix(&self, prefix: impl Into<Cow<'static, str>>) {
        self.progress_tracker.set_pb_prefix(prefix)
    }

    pub(crate) fn set_pb_message(&self, msg: impl Into<Cow<'static, str>>) {
        self.progress_tracker.set_pb_message(msg)
    }

    pub(crate) async fn async_with_progress<F, O>(&self, fut: F) -> O
    where
        F: Future<Output = O>,
    {
        async_with_progress(fut, &self.progress_tracker.progress_bar).await
    }

    /// Does not explicitly return an `Err` for exit code != 0
    pub(crate) async fn exec_fallible_cmd_with_exit_status<S1, S2, I>(
        &self,
        cmd: S1,
        args: I,
    ) -> anyhow::Result<ExitStatus>
    where
        I: IntoIterator<Item = S2>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        let fut = exec_fallible_cmd_with_output(cmd, args);
        Ok(self.async_with_progress(fut).await?.status)
    }

    /// Does explicitly return an `Err` for exit code != 0
    pub(crate) async fn execute_cmd_with_exit_status<S1, S2, I>(
        &self,
        cmd: S1,
        args: I,
    ) -> anyhow::Result<ExitStatus>
    where
        I: IntoIterator<Item = S2>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        let fut = exec_cmd_with_output(cmd, args);
        Ok(self.async_with_progress(fut).await?.status)
    }

    // depends on target
    #[allow(dead_code)]
    pub(crate) async fn exec_fallible_cmd_with_output<S1, S2, I>(
        &self,
        cmd: S1,
        args: I,
    ) -> anyhow::Result<Output>
    where
        I: IntoIterator<Item = S2>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        let fut = exec_fallible_cmd_with_output(cmd, args);
        self.async_with_progress(fut).await
    }

    pub(crate) async fn execute_cmd_with_stdout<S1, S2, I>(
        &self,
        cmd: S1,
        args: I,
    ) -> anyhow::Result<Vec<u8>>
    where
        I: IntoIterator<Item = S2>,
        S1: AsRef<OsStr>,
        S2: AsRef<OsStr>,
    {
        let fut = exec_cmd_with_output(cmd, args);
        Ok(self.async_with_progress(fut).await?.stdout)
    }

    pub(crate) fn unyms(&self, amount: u128) -> Vec<Coin> {
        vec![self.unym(amount)]
    }

    pub(crate) fn unym(&self, amount: u128) -> Coin {
        Coin::new(amount, "unym")
    }
}

pub(crate) struct ProgressTracker {
    start: Instant,
    pub(crate) progress_bar: ProgressBar,
}

impl ProgressTracker {
    pub(crate) fn new<I: AsRef<str>>(msg: I) -> Self {
        // SAFETY: this is a valid template
        let progress_bar = ProgressBar::new_spinner();

        #[allow(clippy::unwrap_used)]
        progress_bar.set_style(ProgressStyle::with_template("{spinner} {prefix} {msg}").unwrap());
        progress_bar.println(style(msg.as_ref()).bold().to_string());

        ProgressTracker {
            start: Instant::now(),
            progress_bar,
        }
    }

    pub(crate) fn println<I: AsRef<str>>(&self, msg: I) {
        if std::io::stdout().is_terminal() {
            self.progress_bar.println(msg)
        } else {
            info!("{}", msg.as_ref());
        }
    }

    pub(crate) fn println_with_emoji<I: AsRef<str>>(&self, msg: I, emoji: &str) {
        self.println(format!("{} {}", Emoji::new(emoji, ""), msg.as_ref()));
    }

    pub(crate) fn set_pb_prefix(&self, prefix: impl Into<Cow<'static, str>>) {
        self.progress_bar.set_prefix(prefix)
    }

    pub(crate) fn set_pb_message(&self, msg: impl Into<Cow<'static, str>>) {
        self.progress_bar.set_message(msg)
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        ProgressTracker {
            start: Instant::now(),
            progress_bar: ProgressBar::new_spinner(),
        }
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        self.println_with_emoji(
            format!("Done in {}", HumanDuration(self.start.elapsed())),
            "✨",
        );
        self.progress_bar.finish_and_clear();
    }
}
