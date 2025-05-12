// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymNodeError;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use nym_task::ShutdownToken;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, trace, warn};

struct ReplayProtectionBackgroundTaskConfig {
    current_bloomfilter_path: PathBuf,
    current_bloomfilter_temp_flush_path: PathBuf,

    false_positive_rate: f64,
    filter_reset_rate: Duration,
    disk_flushing_rate: Duration,
    bloomfilter_size_multiplier: f64,
    minimum_bloomfilter_packets_per_second: usize,
}

impl From<&Config> for ReplayProtectionBackgroundTaskConfig {
    fn from(config: &Config) -> Self {
        todo!()
        // ReplayProtectionBackgroundTaskConfig {
        //     current_bloomfilter_path: config
        //         .mixnet
        //         .replay_protection
        //         .storage_paths
        //         .current_bloomfilter_filepath(),
        //     current_bloomfilter_temp_flush_path: config
        //         .mixnet
        //         .replay_protection
        //         .storage_paths
        //         .current_bloomfilter_being_flushed_filepath(),
        //     false_positive_rate: config.mixnet.replay_protection.debug.false_positive_rate,
        //     filter_reset_rate: config.mixnet.replay_protection.debug.bloomfilter_reset_rate,
        //     disk_flushing_rate: config
        //         .mixnet
        //         .replay_protection
        //         .debug
        //         .bloomfilter_disk_flushing_rate,
        //     bloomfilter_size_multiplier: config
        //         .mixnet
        //         .replay_protection
        //         .debug
        //         .bloomfilter_size_multiplier,
        //     minimum_bloomfilter_packets_per_second: config
        //         .mixnet
        //         .replay_protection
        //         .debug
        //         .bloomfilter_minimum_packets_per_second_size,
        // }
    }
}

// background task responsible for periodically flushing the bloomfilters to disk
pub struct ReplayProtectionDiskFlush {
    config: ReplayProtectionBackgroundTaskConfig,

    filters: ReplayProtectionBloomfilters,
    shutdown_token: ShutdownToken,
}

impl ReplayProtectionDiskFlush {
    pub(crate) async fn new(
        config: &Config,
        primary_key_rotation_id: u32,
        secondary_key_rotation_id: Option<u32>,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        // based on current rotation id, figure out which filter is which and also purge old ones, if exist.
        todo!()
        //
        // let task_config: ReplayProtectionBackgroundTaskConfig = config.into();
        //
        // if task_config.current_bloomfilter_temp_flush_path.exists() {
        //     error!(
        //         "bloomfilter didn't get successfully flushed to disk and its data got corrupted"
        //     );
        //     fs::remove_file(&task_config.current_bloomfilter_temp_flush_path).map_err(|source| {
        //         NymNodeError::BloomfilterIoFailure {
        //             source,
        //             path: task_config.current_bloomfilter_temp_flush_path.clone(),
        //         }
        //     })?
        // }
        //
        // // if there's nothing on disk, we must create a new filter
        // let bloomfilter = if task_config.current_bloomfilter_path.exists() {
        //     ReplayProtectionBloomfilters::load(&task_config.current_bloomfilter_path).await?
        // } else {
        //     let bf_items = items_in_bloomfilter(
        //         task_config.filter_reset_rate,
        //         config
        //             .mixnet
        //             .replay_protection
        //             .debug
        //             .initial_expected_packets_per_second,
        //     );
        //
        //     ReplayProtectionBloomfilters::new_empty(bf_items, task_config.false_positive_rate)?
        // };
        //
        // Ok(ReplayProtectionDiskFlush {
        //     config: task_config,
        //     filters: bloomfilter,
        //     shutdown_token,
        // })
    }

    pub(crate) fn global_bloomfilters(&self) -> ReplayProtectionBloomfilters {
        self.filters.clone()
    }

    async fn flush_to_disk(&self) -> Result<(), NymNodeError> {
        if let Some(temp_parent) = self.config.current_bloomfilter_temp_flush_path.parent() {
            fs::create_dir_all(temp_parent).map_err(|source| {
                NymNodeError::BloomfilterIoFailure {
                    source,
                    path: temp_parent.to_path_buf(),
                }
            })?
        }
        if let Some(current_parent) = self.config.current_bloomfilter_temp_flush_path.parent() {
            fs::create_dir_all(current_parent).map_err(|source| {
                NymNodeError::BloomfilterIoFailure {
                    source,
                    path: current_parent.to_path_buf(),
                }
            })?
        }

        // because it takes a while to actually write the file to disk,
        // we first write bytes to temporary location,
        // and then we move it to the correct path
        let temp = &self.config.current_bloomfilter_temp_flush_path;
        self.filters.flush_to_disk(temp).await?;
        fs::rename(temp, &self.config.current_bloomfilter_path).map_err(|source| {
            NymNodeError::BloomfilterIoFailure {
                source,
                path: self.config.current_bloomfilter_path.clone(),
            }
        })?;
        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        let mut flush_timer = interval(self.config.disk_flushing_rate);
        flush_timer.reset();

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("ReplayProtectionBackgroundTask: Received shutdown");
                    break;
                }
                _ = flush_timer.tick() => {
                    if let Err(err) = self.flush_to_disk().await {
                        error!("failed to flush bloomfilter to disk: {err}")
                    }
                }
            }
        }

        info!("SHUTDOWN: flushing replay detection bloomfilter to disk. this might take a while. DO NOT INTERRUPT THIS PROCESS");
        if let Err(err) = self.flush_to_disk().await {
            warn!("failed to flush replay detection bloom filter on shutdown: {err}");
        }
    }
}
