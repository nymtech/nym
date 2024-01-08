// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::NymvisorError;
use crate::helpers::{init_path, to_hex_string};
use crate::upgrades::types::{DownloadUrl, UpgradeInfo};
use bytes::Buf;
use futures::stream::StreamExt;
use std::io::BufWriter;
use std::path::PathBuf;
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

async fn chunk_download(
    download_url: &DownloadUrl,
    download_target: &PathBuf,
) -> Result<(), NymvisorError> {
    info!(
        "attempting to download the binary from '{}'",
        download_url.url
    );
    let response = reqwest::get(download_url.url.clone())
        .await
        .map_err(|source| NymvisorError::UpgradeDownloadFailure {
            url: download_url.url.clone(),
            source,
        })?;

    let maybe_length = response.content_length();
    let mut source = response.bytes_stream();

    let output_binary = fs::File::create(download_target).map_err(|source| {
        NymvisorError::DaemonBinaryCreationFailure {
            path: download_target.clone(),
            source,
        }
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
                path: download_target.clone(),
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

fn maybe_verify_checksum(
    upgrade_name: String,
    download_url: &DownloadUrl,
    download_target: &PathBuf,
) -> Result<(), NymvisorError> {
    if !download_url.checksum.is_empty() {
        let checksum = download_url
            .checksum_algorithm
            .calculate_file_checksum(download_target)?;
        if checksum != download_url.checksum {
            return Err(NymvisorError::DownloadChecksumFailure {
                upgrade_name,
                encoded_checksum: to_hex_string(&checksum),
                expected_checksum: to_hex_string(&download_url.checksum),
                algorithm: download_url.checksum_algorithm,
            });
        }
    }
    Ok(())
}

pub(super) async fn download_upgrade_binary(
    config: &Config,
    info: &UpgradeInfo,
) -> Result<(), NymvisorError> {
    info!("attempting to download the upgrade binary");
    let download_url = info.get_download_url()?;

    // if the config specifies checksum MUST be verified and it's missing - return an error
    if config.daemon.debug.enforce_download_checksum && download_url.checksum.is_empty() {
        return Err(NymvisorError::MissingDownloadChecksum {
            upgrade_name: info.name.clone(),
        });
    }

    init_path(config.upgrade_binary_dir(&info.name))?;

    let temp_target = config.temp_upgrade_binary(&info.name);
    let target = config.upgrade_binary(&info.name);

    // perform the download
    chunk_download(download_url, &temp_target).await?;

    // if the checksum is available, do verify it
    maybe_verify_checksum(info.name.clone(), download_url, &temp_target)?;

    // if the checksum exists and it matches, move the file to the correct location
    fs::rename(&temp_target, &target).map_err(|source| NymvisorError::DaemonBinaryCopyFailure {
        source_path: temp_target,
        target_path: target,
        source,
    })
}

pub(crate) fn os_arch() -> String {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    // a special case for macos because of course it's its own special snowflake
    if os == "macos" {
        format!("darwin-{arch}")
    } else {
        format!("{os}-{arch}")
    }
}
