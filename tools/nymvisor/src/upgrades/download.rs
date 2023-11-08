// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::NymvisorError;
use crate::upgrades::types::UpgradeInfo;
use bytes::Buf;
use futures::stream::StreamExt;
use std::io::BufWriter;
use std::time::Duration;
use std::{env, fs, io};
use tracing::info;

const LOGGING_RATE: Duration = Duration::from_millis(25);

fn log_progress_bar(downloaded: u64, length: u64) {
    let percentage = downloaded as f32 * 100. / length as f32;

    let width = 40;
    let filled = (percentage * width as f32 / 100.) as usize;
    let empty = width - filled;

    let filled = format!("{:#^width$}", "", width = filled);
    let empty = format!("{: ^width$}", "", width = empty);

    let mb_downloaded = downloaded as f64 / (1024. * 1024.);
    let mb_total = length as f64 / (1024. * 1024.);

    info!("[{filled}{empty}] {mb_downloaded:.2}MB/{mb_total:.2}MB ({percentage:.2}%)");
}

pub(super) async fn download_upgrade_binary(
    config: &Config,
    info: &UpgradeInfo,
) -> Result<(), NymvisorError> {
    info!("attempting to download the upgrade binary");
    let Some(download_url) = info.platforms.get(&os_arch()) else {
        return Err(NymvisorError::NoDownloadUrls {
            upgrade_name: info.name.clone(),
            arch: os_arch(),
        });
    };

    fs::create_dir_all(config.upgrade_binary_dir(&info.name)).map_err(|source| {
        NymvisorError::PathInitFailure {
            path: config.upgrade_binary_dir(&info.name),
            source,
        }
    })?;

    let target = config.upgrade_binary(&info.name);
    let response = reqwest::get(download_url.url.clone())
        .await
        .map_err(|source| NymvisorError::UpgradeDownloadFailure {
            url: download_url.url.clone(),
            source,
        })?;

    let maybe_length = response.content_length();
    let mut source = response.bytes_stream();

    let output_binary =
        fs::File::create(&target).map_err(|source| NymvisorError::DaemonBinaryCreationFailure {
            path: target.clone(),
            source,
        })?;
    let mut out = BufWriter::new(output_binary);

    info!("beginning the download");
    let mut downloaded = 0;
    let mut last_logged = tokio::time::Instant::now();
    while let Some(chunk) = source.next().await {
        let mut bytes = chunk
            .map_err(|err_source| NymvisorError::UpgradeDownloadFailure {
                url: download_url.url.clone(),
                source: err_source,
            })?
            .reader();

        downloaded += io::copy(&mut bytes, &mut out).map_err(|err_source| {
            NymvisorError::DaemonBinaryCreationFailure {
                path: target.clone(),
                source: err_source,
            }
        })?;

        if let Some(length) = maybe_length {
            if last_logged.elapsed() > LOGGING_RATE {
                log_progress_bar(downloaded, length);
                last_logged = tokio::time::Instant::now();
            }
        }
    }
    if let Some(length) = maybe_length {
        log_progress_bar(length, length)
    }
    info!("finished the download");

    Ok(())
}

fn os_arch() -> String {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    // a special case for macos because of course it's its own special snowflake
    if os == "macos" {
        format!("darwin-{arch}")
    } else {
        format!("{os}-{arch}")
    }
}
