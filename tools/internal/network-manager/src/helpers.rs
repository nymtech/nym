// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use indicatif::ProgressBar;
use nym_config::{must_get_home, NYM_DIR};
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tokio::pin;
use tokio::time::interval;

pub(crate) fn default_db_file() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join("network-manager")
        .join("network-data.sqlite")
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
