// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NetworkManagerError;
use indicatif::ProgressBar;
use nym_config::{must_get_home, NYM_DIR};
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::pin;
use tokio::time::interval;

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
