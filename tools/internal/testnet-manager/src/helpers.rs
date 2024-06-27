// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NetworkManagerError;
use indicatif::{HumanDuration, ProgressBar};
use nym_config::{must_get_home, NYM_DIR};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::pin;
use tokio::time::interval;

// struct Ctx<'a, T> {
//     progress: ProgressTracker,
//     network: LoadedNetwork<'a>,
//     inner: T,
// }

pub(crate) trait ProgressCtx {
    fn progress_tracker(&self) -> &ProgressTracker;

    fn println<I: AsRef<str>>(&self, msg: I) {
        self.progress_tracker().println(msg)
    }

    fn set_pb_prefix(&self, prefix: impl Into<Cow<'static, str>>) {
        self.progress_tracker().set_pb_prefix(prefix)
    }

    fn set_pb_message(&self, msg: impl Into<Cow<'static, str>>) {
        self.progress_tracker().set_pb_message(msg)
    }

    async fn async_with_progress<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T>,
    {
        async_with_progress(fut, &self.progress_tracker().progress_bar).await
    }
}

// pub(crate) trait NetworkCtx {
//     fn loaded_network(&self) -> &LoadedNetwork;
// }

#[derive(Serialize, Deserialize)]
pub struct RunCommands(pub(crate) Vec<String>);

impl Display for RunCommands {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for cmd in &self.0 {
            writeln!(f, "{cmd}")?
        }
        Ok(())
    }
}

pub(crate) struct ProgressTracker {
    start: Instant,
    pub(crate) progress_bar: ProgressBar,
}

impl ProgressTracker {
    pub(crate) fn new<I: AsRef<str>>(msg: I) -> Self {
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.println(msg);

        ProgressTracker {
            start: Instant::now(),
            progress_bar,
        }
    }

    pub(crate) fn println<I: AsRef<str>>(&self, msg: I) {
        self.progress_bar.println(msg)
    }

    pub(crate) fn set_pb_prefix(&self, prefix: impl Into<Cow<'static, str>>) {
        self.progress_bar.set_prefix(prefix)
    }

    pub(crate) fn set_pb_message(&self, msg: impl Into<Cow<'static, str>>) {
        self.progress_bar.set_message(msg)
    }

    pub(crate) fn output_run_commands(&self, cmds: &RunCommands) {
        self.println("🏇 run the binaries with the following commands:");
        for cmd in &cmds.0 {
            self.println(cmd)
        }
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
        self.progress_bar.println(format!(
            "✨ Done in {}",
            HumanDuration(self.start.elapsed())
        ));
        self.progress_bar.finish_and_clear();
    }
}

pub(crate) fn default_storage_dir() -> PathBuf {
    must_get_home().join(NYM_DIR).join("testnet-manager")
}

pub(crate) fn default_db_file() -> PathBuf {
    default_storage_dir().join("network-data.sqlite")
}

pub(crate) async fn async_with_progress<F, T>(fut: F, pb: &ProgressBar) -> T
where
    F: Future<Output = T>,
{
    pb.tick();
    pin!(fut);
    let mut update_interval = interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            _ = update_interval.tick() => {
                pb.tick()
            }
            res = &mut fut => {
                return res
            }
        }
    }
}

pub(crate) fn wasm_code<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, NetworkManagerError> {
    let path = path.as_ref();
    assert!(path.exists());
    let mut file = std::fs::File::open(path)?;
    let mut data = Vec::new();

    file.read_to_end(&mut data)?;
    Ok(data)
}
