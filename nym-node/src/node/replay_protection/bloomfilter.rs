// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use bloomfilter::Bloom;
use human_repr::HumanDuration;
use nym_sphinx_types::REPLAY_TAG_SIZE;
use std::collections::HashMap;
use std::mem;
use std::path::Path;
use std::sync::{Arc, PoisonError, TryLockError};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::Instant;
use tracing::{debug, error, info};

// auxiliary data associated with the bloomfilter to get some statistics from the time of its creation
// this is needed in order to more accurately resize it upon reset
#[derive(Copy, Clone)]
pub(crate) struct ReplayProtectionBloomfilterMetadata {
    // used in the unlikely case of epoch durations being changed. it doesn't really cost us anything
    // to include it, so might as well
    pub(crate) creation_time: Instant,

    /// Number of packets that this node has received since startup, as recorded when this bloomfilter was created.
    /// Used for determining the approximate packet rate and thus number of entries in the bloomfilter
    pub(crate) packets_received_at_creation: usize,

    pub(crate) rotation_id: u32,
}

// it appears that now std Mutex is faster (or comparable) to parking_lot
// in high contention situations: https://github.com/rust-lang/rust/pull/95035#issuecomment-1073966631
// (tokio's async Mutex has too much overhead due to the number of access required)
#[derive(Clone)]
pub(crate) struct ReplayProtectionBloomfilters {
    disabled: bool,
    inner: Arc<std::sync::Mutex<ReplayProtectionBloomfiltersInner>>,
}

impl ReplayProtectionBloomfilters {
    pub(crate) fn new_empty(items_count: usize, fp_p: f64) -> Result<Self, NymNodeError> {
        todo!()
        // Ok(ReplayProtectionBloomfilter {
        //     disabled: false,
        //     inner: Arc::new(std::sync::Mutex::new(ReplayProtectionBloomfilterInner {
        //         current_filter: Bloom::new_for_fp_rate(items_count, fp_p)
        //             .map_err(NymNodeError::bloomfilter_failure)?,
        //     })),
        // })
    }

    // SAFETY: the hardcoded values of 1,1 are valid
    #[allow(clippy::unwrap_used)]
    pub(crate) fn new_disabled() -> Self {
        // well, technically it's not fully empty, but the memory footprint is negligible
        ReplayProtectionBloomfilters {
            disabled: true,
            inner: Arc::new(std::sync::Mutex::new(ReplayProtectionBloomfiltersInner {
                primary: RotationFilter {
                    metadata: ReplayProtectionBloomfilterMetadata {
                        creation_time: Instant::now(),
                        packets_received_at_creation: 0,
                        rotation_id: u32::MAX,
                    },
                    data: Bloom::new(1, 1).unwrap(),
                },
                secondary: None,
                pre_announced: None,
            })),
        }
    }

    pub(crate) fn disabled(&self) -> bool {
        self.disabled
    }

    pub(crate) fn allocate_pre_announced(
        &self,
        items_count: usize,
        fp_p: f64,
        packets_received_at_creation: usize,
        rotation_id: u32,
    ) -> Result<(), NymNodeError> {
        // build the new filter
        let filter =
            Bloom::new_for_fp_rate(items_count, fp_p).map_err(NymNodeError::bloomfilter_failure)?;

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;

        guard.pre_announced = Some(RotationFilter {
            metadata: ReplayProtectionBloomfilterMetadata {
                creation_time: Instant::now(),
                packets_received_at_creation,
                rotation_id,
            },
            data: filter,
        });
        Ok(())
    }

    pub(crate) fn promote_pre_announced(&self) -> Result<(), NymNodeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;

        let Some(mut pre_announced) = guard.pre_announced.take() else {
            error!("there was no pre-announced bloomfilter to promote");
            return Ok(());
        };

        // pre_announced -> primary
        // primary -> temp (pre_announced)
        mem::swap(&mut guard.primary, &mut pre_announced);

        // temp (pre_announced) -> secondary
        guard.secondary = Some(pre_announced);
        Ok(())
    }

    pub(crate) fn purge_secondary(&self) -> Result<(), NymNodeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;
        guard.secondary = None;
        Ok(())
    }

    pub(crate) fn primary_metadata(
        &self,
    ) -> Result<ReplayProtectionBloomfilterMetadata, NymNodeError> {
        let metadata = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?
            .primary
            .metadata;

        Ok(metadata)
    }

    pub(crate) fn reset(&self, items_count: usize, fp_p: f64) -> Result<(), NymNodeError> {
        // 1. build the new filter
        todo!()
        // let new_inner = ReplayProtectionBloomfilterInner {
        //     current_filter: Bloom::new_for_fp_rate(items_count, fp_p)
        //         .map_err(NymNodeError::bloomfilter_failure)?,
        // };
        //
        // // 2. swap it
        // let mut guard = self
        //     .inner
        //     .lock()
        //     .map_err(|_| NymNodeError::BloomfilterFailure {
        //         message: "mutex got poisoned",
        //     })?;
        //
        // *guard = new_inner;
        // Ok(())
    }

    // NOTE: with key rotations we'll have to check whether the file is still valid and which
    // key it corresponds to, but that's a future problem
    pub(crate) async fn load<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        todo!()
        // info!("attempting to load prior replay detection bloomfilter...");
        // let path = path.as_ref();
        // let mut file =
        //     File::open(path)
        //         .await
        //         .map_err(|source| NymNodeError::BloomfilterIoFailure {
        //             source,
        //             path: path.to_path_buf(),
        //         })?;
        //
        // let mut buf = Vec::new();
        // file.read_to_end(&mut buf)
        //     .await
        //     .map_err(|source| NymNodeError::BloomfilterIoFailure {
        //         source,
        //         path: path.to_path_buf(),
        //     })?;
        //
        // Ok(ReplayProtectionBloomfilter {
        //     disabled: false,
        //     inner: Arc::new(std::sync::Mutex::new(ReplayProtectionBloomfilterInner {
        //         current_filter: Bloom::from_bytes(buf)
        //             .map_err(NymNodeError::bloomfilter_failure)?,
        //     })),
        // })
    }

    // average HDD has the write speed of ~80MB/s so a 2GB bloomfilter would take almost 30s to write...
    // and this function is explicitly async and using tokio's async operations, because otherwise
    // we'd have to go through the whole hassle of using spawn_blocking and awaiting that one instead
    pub(crate) async fn flush_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), NymNodeError> {
        todo!()
        // debug!("flushing replay protection bloomfilter to disk...");
        // let start = Instant::now();
        // let path = path.as_ref();
        //
        // let mut file =
        //     File::create(path)
        //         .await
        //         .map_err(|source| NymNodeError::BloomfilterIoFailure {
        //             source,
        //             path: path.to_path_buf(),
        //         })?;
        // let data = self.bytes().map_err(|_| NymNodeError::BloomfilterFailure {
        //     message: "mutex got poisoned",
        // })?;
        // file.write_all(&data)
        //     .await
        //     .map_err(|source| NymNodeError::BloomfilterIoFailure {
        //         source,
        //         path: path.to_path_buf(),
        //     })?;
        //
        // let elapsed = start.elapsed();
        //
        // info!(
        //     "flushed replay protection bloomfilter to disk. it took: {}",
        //     elapsed.human_duration()
        // );
        //
        // Ok(())
    }
}

struct RotationFilter {
    metadata: ReplayProtectionBloomfilterMetadata,
    data: Bloom<[u8; REPLAY_TAG_SIZE]>,
}

impl ReplayProtectionBloomfilters {
    pub(crate) fn batch_try_check_and_set(
        &self,
        reply_tags: &HashMap<u32, Vec<&[u8; REPLAY_TAG_SIZE]>>,
    ) -> Option<Result<HashMap<u32, Vec<bool>>, PoisonError<()>>> {
        let mut guard = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(_)) => return Some(Err(PoisonError::new(()))),
            Err(TryLockError::WouldBlock) => return None,
        };

        Some(Ok(guard.batch_check_and_set(&reply_tags)))
    }

    pub(crate) fn batch_check_and_set(
        &self,
        reply_tags: &HashMap<u32, Vec<&[u8; REPLAY_TAG_SIZE]>>,
    ) -> Result<HashMap<u32, Vec<bool>>, PoisonError<()>> {
        let Ok(mut guard) = self.inner.lock() else {
            return Err(PoisonError::new(()));
        };

        Ok(guard.batch_check_and_set(&reply_tags))
    }

    // due to the size of the bloomfilter, extra caution has to be applied when using this method
    // note: we're not getting reference to bytes as this method is used when flushing data to the disk
    // (which takes ~30s) and we can't block the mutex for that long.
    fn bytes(&self) -> Result<Vec<u8>, PoisonError<()>> {
        todo!()
        // let guard = self.inner.lock().map_err(|_| PoisonError::new(()))?;
        // Ok(guard.current_filter.to_bytes())
    }
}

struct ReplayProtectionBloomfiltersInner {
    primary: RotationFilter,

    // don't worry, we'll never have 3 active filters at once,
    // we will either have a secondary (during the first epoch of a new rotation)
    // or a pre_announced (during the last epoch of the current rotation)
    // during epoch transition, the following change will happen:
    // primary -> secondary
    // pre_announced -> primary
    // I'm not using an enum because it's easier to reason about those as separate fields
    secondary: Option<RotationFilter>,
    pre_announced: Option<RotationFilter>,
}

impl ReplayProtectionBloomfiltersInner {
    fn batch_check_and_set(
        &mut self,
        reply_tags: &HashMap<u32, Vec<&[u8; REPLAY_TAG_SIZE]>>,
    ) -> HashMap<u32, Vec<bool>> {
        let mut result = HashMap::with_capacity(reply_tags.len());
        for (&rotation_id, reply_tags) in reply_tags {
            // try to 'find' the relevant filter. we might be doing 3 reads here, but realistically it's
            // going to be 'primary' most of the time and even if not, it's just few ns of overhead...
            let mut filter = if self.primary.metadata.rotation_id == rotation_id {
                Some(&mut self.primary.data)
            } else if let Some(secondary) = &mut self.secondary {
                // if let chaining won't be stable until 1.88 so we have to do the Option workaround
                if secondary.metadata.rotation_id == rotation_id {
                    Some(&mut secondary.data)
                } else {
                    None
                }
            } else if let Some(pre_announced) = &mut self.pre_announced {
                if pre_announced.metadata.rotation_id == rotation_id {
                    Some(&mut pre_announced.data)
                } else {
                    None
                }
            } else {
                None
            };

            let Some(mut filter) = filter else {
                // if we've received a packet from an unknown rotation, it most likely means it has been replayed
                // from an older rotation, so mark it as such
                result.insert(rotation_id, vec![false; reply_tags.len()]);
                continue;
            };

            let mut rotation_results = Vec::with_capacity(reply_tags.len());
            for tag in reply_tags {
                rotation_results.push(filter.check_and_set(tag))
            }
            result.insert(rotation_id, rotation_results);
        }

        result
    }
}
