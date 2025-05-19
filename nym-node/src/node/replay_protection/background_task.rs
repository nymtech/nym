// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::{
    DEFAULT_RD_BLOOMFILTER_FILE_EXT, DEFAULT_RD_BLOOMFILTER_FLUSH_FILE_EXT,
};
use crate::config::Config;
use crate::error::NymNodeError;
use crate::node::replay_protection::bloomfilter::RotationFilter;
use crate::node::replay_protection::helpers::parse_rotation_id_from_filename;
use crate::node::replay_protection::items_in_bloomfilter;
use crate::node::replay_protection::manager::ReplayProtectionBloomfiltersManager;
use human_repr::HumanDuration;
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownToken;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval, Instant};
use tracing::{debug, error, info, trace, warn};

// background task responsible for periodically flushing the bloomfilters to disk
pub struct ReplayProtectionDiskFlush {
    bloomfilters_directory: PathBuf,
    disk_flushing_rate: Duration,

    filters_manager: ReplayProtectionBloomfiltersManager,
    shutdown_token: ShutdownToken,
}

impl ReplayProtectionDiskFlush {
    pub(crate) async fn new(
        config: &Config,
        primary_key_rotation_id: u32,
        secondary_key_rotation_id: Option<u32>,
        metrics: NymNodeMetrics,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        let bloomfilters_directory = config
            .mixnet
            .replay_protection
            .storage_paths
            .current_bloomfilters_directory
            .clone();

        let dir_read_err = |source| NymNodeError::BloomfilterIoFailure {
            source,
            path: bloomfilters_directory.clone(),
        };

        if !bloomfilters_directory.exists() {
            fs::create_dir_all(&bloomfilters_directory).map_err(dir_read_err)?;
        }

        let available_filters_dir = fs::read_dir(&bloomfilters_directory).map_err(dir_read_err)?;

        // figure out what bloomfilters we have available on disk
        let mut filter_files = HashMap::new();
        for entry in available_filters_dir.into_iter() {
            let entry = entry.map_err(dir_read_err)?;
            let path = entry.path();

            let Some(rotation) = entry
                .file_name()
                .to_str()
                .and_then(parse_rotation_id_from_filename)
            else {
                warn!("invalid bloomfilter file at '{}'", path.display());
                continue;
            };

            // if any bloomfilter has the temp extension, we can't trust its data as it hasn't completed the flush
            if let Some(ext) = entry.path().extension() {
                if ext == DEFAULT_RD_BLOOMFILTER_FLUSH_FILE_EXT {
                    error!(
                        "bloomfilter {rotation} didn't get successfully flushed to disk and its data got corrupted"
                    );
                    fs::remove_file(&path)
                        .map_err(|source| NymNodeError::BloomfilterIoFailure { source, path })?;
                    continue;
                }
            }

            filter_files.insert(rotation, path);
        }

        let rebuild_items_in_filter = items_in_bloomfilter(
            Duration::from_secs(25 * 60 * 60),
            config
                .mixnet
                .replay_protection
                .debug
                .initial_expected_packets_per_second,
        );
        let fp_r = config.mixnet.replay_protection.debug.false_positive_rate;

        // if filters do not exist on disk, we must make new ones
        let primary_bloomfilter = match filter_files.get(&primary_key_rotation_id) {
            Some(primary_path) => RotationFilter::load(primary_path)?,
            None => {
                info!("no stored bloomfilter for rotation {primary_key_rotation_id}");
                RotationFilter::new(rebuild_items_in_filter, fp_r, 0, primary_key_rotation_id)?
            }
        };

        let secondary_bloomfilter =
            if let Some(secondary_key_rotation_id) = secondary_key_rotation_id {
                match filter_files.get(&secondary_key_rotation_id) {
                    Some(secondary_path) => Some(RotationFilter::load(secondary_path)?),
                    None => {
                        info!("no stored bloomfilter for rotation {secondary_key_rotation_id}");
                        Some(RotationFilter::new(
                            rebuild_items_in_filter,
                            fp_r,
                            0,
                            secondary_key_rotation_id,
                        )?)
                    }
                }
            } else {
                None
            };

        Ok(ReplayProtectionDiskFlush {
            bloomfilters_directory,
            disk_flushing_rate: config
                .mixnet
                .replay_protection
                .debug
                .bloomfilter_disk_flushing_rate,
            filters_manager: ReplayProtectionBloomfiltersManager::new(
                config,
                primary_bloomfilter,
                secondary_bloomfilter,
                metrics,
            ),
            shutdown_token,
        })
    }

    fn bloomfilter_filepath(&self, rotation_id: u32) -> PathBuf {
        self.bloomfilters_directory
            .join(format!("rot-{rotation_id}"))
            .with_extension(DEFAULT_RD_BLOOMFILTER_FILE_EXT)
    }

    fn current_bloomfilter_being_flushed_filepath(&self, rotation_id: u32) -> PathBuf {
        self.bloomfilters_directory
            .join(format!("rot-{rotation_id}"))
            .with_extension(DEFAULT_RD_BLOOMFILTER_FLUSH_FILE_EXT)
    }

    pub(crate) fn bloomfilters_manager(&self) -> ReplayProtectionBloomfiltersManager {
        self.filters_manager.clone()
    }

    async fn flush(&self, data: Vec<u8>, rotation_id: u32) -> Result<(), NymNodeError> {
        // because it takes a while to actually write the file to disk,
        // we first write bytes to temporary location,
        // and then we move it to the correct path
        let temp_path = self.current_bloomfilter_being_flushed_filepath(rotation_id);
        let final_path = self.bloomfilter_filepath(rotation_id);
        debug!("flushing replay protection bloomfilter {rotation_id} to disk...");
        let start = Instant::now();

        let mut file = File::create(&temp_path).await.map_err(|source| {
            NymNodeError::BloomfilterIoFailure {
                source,
                path: temp_path.clone(),
            }
        })?;

        file.write_all(&data)
            .await
            .map_err(|source| NymNodeError::BloomfilterIoFailure {
                source,
                path: temp_path.to_path_buf(),
            })?;

        fs::rename(temp_path, &final_path).map_err(|source| {
            NymNodeError::BloomfilterIoFailure {
                source,
                path: final_path,
            }
        })?;

        let elapsed = start.elapsed();

        info!(
            "flushed replay protection bloomfilter {rotation_id} to disk. it took: {}",
            elapsed.human_duration()
        );

        Ok(())
    }

    // average HDD has the write speed of ~80MB/s so a 2GB bloomfilter would take almost 30s to write...
    // and this function is explicitly async and using tokio's async operations, because otherwise
    // we'd have to go through the whole hassle of using spawn_blocking and awaiting that one instead
    async fn flush_primary(&self) -> Result<(), NymNodeError> {
        let (bytes, id) = self.filters_manager.primary_bytes_and_id()?;
        self.flush(bytes, id).await
    }

    async fn flush_secondary(&self) -> Result<(), NymNodeError> {
        let Some((bytes, id)) = self.filters_manager.secondary_bytes_and_id()? else {
            return Ok(());
        };
        self.flush(bytes, id).await
    }

    async fn flush_filters_to_disk(&self) -> Result<(), NymNodeError> {
        if let Some(parent) = self.bloomfilters_directory.parent() {
            fs::create_dir_all(parent).map_err(|source| NymNodeError::BloomfilterIoFailure {
                source,
                path: parent.to_path_buf(),
            })?
        }

        self.flush_primary().await?;
        self.flush_secondary().await?;

        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        let mut flush_timer = interval(self.disk_flushing_rate);
        flush_timer.reset();

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("ReplayProtectionBackgroundTask: Received shutdown");
                    break;
                }
                _ = flush_timer.tick() => {
                    if let Err(err) = self.flush_filters_to_disk().await {
                        error!("failed to flush bloomfilter to disk: {err}")
                    }
                }
            }
        }

        info!("SHUTDOWN: flushing replay detection bloomfilter to disk. this might take a while. DO NOT INTERRUPT THIS PROCESS");
        if let Err(err) = self.flush_filters_to_disk().await {
            warn!("failed to flush replay detection bloom filters on shutdown: {err}");
        }
    }
}
