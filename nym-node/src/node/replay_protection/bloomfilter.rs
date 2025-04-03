// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use bloomfilter::Bloom;
use human_repr::HumanDuration;
use nym_sphinx_types::REPLAY_TAG_SIZE;
use std::path::Path;
use std::sync::{Arc, PoisonError, TryLockError};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::Instant;
use tracing::{debug, info};

// it appears that now std Mutex is faster (or comparable) to parking_lot
// in high contention situations: https://github.com/rust-lang/rust/pull/95035#issuecomment-1073966631
// (tokio's async Mutex has too much overhead due to the number of access required)
#[derive(Clone)]
pub(crate) struct ReplayProtectionBloomfilter {
    inner: Arc<std::sync::Mutex<ReplayProtectionBloomfilterInner>>,
}

impl ReplayProtectionBloomfilter {
    pub(crate) fn new_empty(items_count: usize, fp_p: f64) -> Result<Self, NymNodeError> {
        Ok(ReplayProtectionBloomfilter {
            inner: Arc::new(std::sync::Mutex::new(ReplayProtectionBloomfilterInner {
                current_filter: Bloom::new_for_fp_rate(items_count, fp_p)
                    .map_err(NymNodeError::bloomfilter_failure)?,
            })),
        })
    }

    pub(crate) fn reset(&self, items_count: usize, fp_p: f64) -> Result<(), NymNodeError> {
        // 1. build the new filter
        let new_inner = ReplayProtectionBloomfilterInner {
            current_filter: Bloom::new_for_fp_rate(items_count, fp_p)
                .map_err(NymNodeError::bloomfilter_failure)?,
        };

        // 2. swap it
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;

        *guard = new_inner;
        Ok(())
    }

    // NOTE: with key rotations we'll have to check whether the file is still valid and which
    // key it corresponds to, but that's a future problem
    pub(crate) async fn load<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        info!("attempting to load prior replay detection bloomfilter...");
        let path = path.as_ref();
        let mut file =
            File::open(path)
                .await
                .map_err(|source| NymNodeError::BloomfilterIoFailure {
                    source,
                    path: path.to_path_buf(),
                })?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .await
            .map_err(|source| NymNodeError::BloomfilterIoFailure {
                source,
                path: path.to_path_buf(),
            })?;

        Ok(ReplayProtectionBloomfilter {
            inner: Arc::new(std::sync::Mutex::new(ReplayProtectionBloomfilterInner {
                current_filter: Bloom::from_bytes(buf)
                    .map_err(NymNodeError::bloomfilter_failure)?,
            })),
        })
    }

    // average HDD has the write speed of ~80MB/s so a 2GB bloomfilter would take almost 30s to write...
    // and this function is explicitly async and using tokio's async operations, because otherwise
    // we'd have to go through the whole hassle of using spawn_blocking and awaiting that one instead
    pub(crate) async fn flush_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), NymNodeError> {
        debug!("flushing replay protection bloomfilter to disk...");
        let start = Instant::now();
        let path = path.as_ref();

        let mut file =
            File::create(path)
                .await
                .map_err(|source| NymNodeError::BloomfilterIoFailure {
                    source,
                    path: path.to_path_buf(),
                })?;
        let data = self.bytes().map_err(|_| NymNodeError::BloomfilterFailure {
            message: "mutex got poisoned",
        })?;
        file.write_all(&data)
            .await
            .map_err(|source| NymNodeError::BloomfilterIoFailure {
                source,
                path: path.to_path_buf(),
            })?;

        let elapsed = start.elapsed();

        info!(
            "flushed replay protection bloomfilter to disk. it took: {}",
            elapsed.human_duration()
        );

        Ok(())
    }
}

struct ReplayProtectionBloomfilterInner {
    // metadata to do with epochs, etc.
    current_filter: Bloom<[u8; REPLAY_TAG_SIZE]>,
    // overlap_filter: bloomfilter::Bloom<[u8; REPLAY_TAG_SIZE]>,
}

impl ReplayProtectionBloomfilter {
    #[allow(dead_code)]
    pub(crate) fn check_and_set(
        &self,
        replay_tag: &[u8; REPLAY_TAG_SIZE],
    ) -> Result<bool, PoisonError<()>> {
        let Ok(mut guard) = self.inner.lock() else {
            return Err(PoisonError::new(()));
        };

        Ok(guard.current_filter.check_and_set(replay_tag))
    }

    #[allow(dead_code)]
    pub(crate) fn try_check_and_set(
        &self,
        replay_tag: &[u8; REPLAY_TAG_SIZE],
    ) -> Option<Result<bool, PoisonError<()>>> {
        let mut guard = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(_)) => return Some(Err(PoisonError::new(()))),
            Err(TryLockError::WouldBlock) => return None,
        };

        Some(Ok(guard.current_filter.check_and_set(replay_tag)))
    }

    pub(crate) fn batch_try_check_and_set(
        &self,
        reply_tags: &[&[u8; REPLAY_TAG_SIZE]],
    ) -> Option<Result<Vec<bool>, PoisonError<()>>> {
        let mut guard = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(_)) => return Some(Err(PoisonError::new(()))),
            Err(TryLockError::WouldBlock) => return None,
        };

        let todo = "";

        let mut result = Vec::with_capacity(reply_tags.len());
        for tag in reply_tags {
            result.push(guard.current_filter.check_and_set(tag));
        }
        return Some(Ok(vec![false; reply_tags.len()]));

        // Ok(result)
    }

    pub(crate) fn batch_check_and_set(
        &self,
        reply_tags: &[&[u8; REPLAY_TAG_SIZE]],
    ) -> Result<Vec<bool>, PoisonError<()>> {
        let Ok(mut guard) = self.inner.lock() else {
            return Err(PoisonError::new(()));
        };

        let todo = "";

        let mut result = Vec::with_capacity(reply_tags.len());
        for tag in reply_tags {
            result.push(guard.current_filter.check_and_set(tag));
        }
        return Ok(vec![false; reply_tags.len()]);

        // Ok(result)
    }

    #[allow(dead_code)]
    pub(crate) fn clear(&self) -> Result<(), PoisonError<()>> {
        let mut guard = self.inner.lock().map_err(|_| PoisonError::new(()))?;
        guard.current_filter.clear();
        Ok(())
    }

    // due to the size of the bloomfilter, extra caution has to be applied when using this method
    // note: we're not getting reference to bytes as this method is used when flushing data to the disk
    // (which takes ~30s) and we can't block the mutex for that long.
    fn bytes(&self) -> Result<Vec<u8>, PoisonError<()>> {
        let guard = self.inner.lock().map_err(|_| PoisonError::new(()))?;
        Ok(guard.current_filter.to_bytes())
    }
}
