// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use bloomfilter::Bloom;
use nym_sphinx_types::REPLAY_TAG_SIZE;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::mem;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError, TryLockError};
use time::OffsetDateTime;
use tracing::{error, info, warn};

// auxiliary data associated with the bloomfilter to get some statistics from the time of its creation
// this is needed in order to more accurately resize it upon reset

#[derive(Copy, Clone)]
pub(crate) struct ReplayProtectionBloomfilterMetadata {
    // used in the unlikely case of epoch durations being changed. it doesn't really cost us anything
    // to include it, so might as well
    pub(crate) creation_time: OffsetDateTime,

    /// Number of packets that this node has received since startup, as recorded when this bloomfilter was created.
    /// Used for determining the approximate packet rate and thus number of entries in the bloomfilter
    pub(crate) packets_received_at_creation: usize,

    pub(crate) rotation_id: u32,
}

impl ReplayProtectionBloomfilterMetadata {
    const SERIALIZED_LEN: usize = size_of::<i64>() + size_of::<u64>() + size_of::<u32>();

    // UNIX_TIMESTAMP || PACKETS_RECEIVED || ROTATION_ID
    pub(crate) fn bytes(&self) -> Vec<u8> {
        self.creation_time
            .unix_timestamp()
            .to_be_bytes()
            .into_iter()
            .chain((self.packets_received_at_creation as u64).to_be_bytes())
            .chain(self.rotation_id.to_be_bytes())
            .collect()
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, NymNodeError> {
        if bytes.len() != Self::SERIALIZED_LEN {
            return Err(NymNodeError::BloomfilterMetadataDeserialisationFailure);
        }

        // SAFETY: we just checked we have correct number of bytes
        #[allow(clippy::unwrap_used)]
        let creation_timestamp = i64::from_be_bytes(bytes[0..8].try_into().unwrap());

        #[allow(clippy::unwrap_used)]
        let packets_received_at_creation =
            u64::from_be_bytes(bytes[8..16].try_into().unwrap()) as usize;

        #[allow(clippy::unwrap_used)]
        let rotation_id = u32::from_be_bytes(bytes[16..].try_into().unwrap());

        Ok(ReplayProtectionBloomfilterMetadata {
            creation_time: OffsetDateTime::from_unix_timestamp(creation_timestamp)
                .map_err(|_| NymNodeError::BloomfilterMetadataDeserialisationFailure)?,
            packets_received_at_creation,
            rotation_id,
        })
    }
}

// it appears that now std Mutex is faster (or comparable) to parking_lot
// in high contention situations: https://github.com/rust-lang/rust/pull/95035#issuecomment-1073966631
// (tokio's async Mutex has too much overhead due to the number of access required)
#[derive(Clone)]
pub(crate) struct ReplayProtectionBloomfilters {
    disabled: bool,
    inner: Arc<Mutex<ReplayProtectionBloomfiltersInner>>,
}

impl ReplayProtectionBloomfilters {
    pub(crate) fn new(primary: RotationFilter, secondary: Option<RotationFilter>) -> Self {
        // figure out if the secondary filter is the overlap or pre_announced filter
        let primary_id = primary.metadata.rotation_id;
        let (overlap, pre_announced) = match secondary {
            None => (None, None),
            Some(secondary_filter) => {
                let secondary_id = secondary_filter.metadata.rotation_id;
                if secondary_id == primary_id + 1 {
                    (None, Some(secondary_filter))
                } else if secondary_id == primary_id - 1 {
                    (Some(secondary_filter), None)
                } else {
                    warn!("{secondary_id} is not valid for either pre_announced or overlap bloomfilter given primary rotation of {primary_id}");
                    (None, None)
                }
            }
        };

        ReplayProtectionBloomfilters {
            disabled: false,
            inner: Arc::new(Mutex::new(ReplayProtectionBloomfiltersInner {
                primary,
                overlap,
                pre_announced,
            })),
        }
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
                        creation_time: OffsetDateTime::now_utc(),
                        packets_received_at_creation: 0,
                        rotation_id: u32::MAX,
                    },
                    data: Bloom::new(1, 1).unwrap(),
                },
                overlap: None,
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
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;

        guard.pre_announced = Some(RotationFilter::new(
            items_count,
            fp_p,
            packets_received_at_creation,
            rotation_id,
        )?);
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
        guard.overlap = Some(pre_announced);
        Ok(())
    }

    pub(crate) fn purge_secondary(&self) -> Result<(), NymNodeError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;
        guard.overlap = None;
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

    pub(crate) fn primary_bytes_and_id(&self) -> Result<(Vec<u8>, u32), NymNodeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;

        let id = guard.primary.metadata.rotation_id;
        let bytes = guard.primary.bytes();
        Ok((bytes, id))
    }

    pub(crate) fn secondary_bytes_and_id(&self) -> Result<Option<(Vec<u8>, u32)>, NymNodeError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| NymNodeError::BloomfilterFailure {
                message: "mutex got poisoned",
            })?;

        let secondary = match guard.overlap.as_ref() {
            Some(overlap) => overlap,
            None => {
                let Some(pre_announced) = guard.pre_announced.as_ref() else {
                    return Ok(None);
                };
                pre_announced
            }
        };

        let id = secondary.metadata.rotation_id;
        let bytes = secondary.bytes();
        Ok(Some((bytes, id)))
    }
}

// map from particular rotation id to vector of results, based on the order of requests received
type BatchCheckResult = HashMap<u32, Vec<bool>>;

impl ReplayProtectionBloomfilters {
    pub(crate) fn batch_try_check_and_set(
        &self,
        reply_tags: &HashMap<u32, Vec<&[u8; REPLAY_TAG_SIZE]>>,
    ) -> Option<Result<BatchCheckResult, PoisonError<()>>> {
        let mut guard = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(_)) => return Some(Err(PoisonError::new(()))),
            Err(TryLockError::WouldBlock) => return None,
        };

        Some(Ok(guard.batch_check_and_set(reply_tags)))
    }

    pub(crate) fn batch_check_and_set(
        &self,
        reply_tags: &HashMap<u32, Vec<&[u8; REPLAY_TAG_SIZE]>>,
    ) -> Result<HashMap<u32, Vec<bool>>, PoisonError<()>> {
        let Ok(mut guard) = self.inner.lock() else {
            return Err(PoisonError::new(()));
        };

        Ok(guard.batch_check_and_set(reply_tags))
    }
}

struct ReplayProtectionBloomfiltersInner {
    primary: RotationFilter,

    // don't worry, we'll never have 3 active filters at once,
    // we will either have a overlap (during the first epoch of a new rotation)
    // or a pre_announced (during the last epoch of the current rotation)
    // during epoch transition, the following change will happen:
    // primary -> overlap
    // pre_announced -> primary
    // I'm not using an enum because it's easier to reason about those as separate fields
    overlap: Option<RotationFilter>,
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
            let filter = if self.primary.metadata.rotation_id == rotation_id {
                Some(&mut self.primary.data)
            } else if let Some(secondary) = &mut self.overlap {
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

            let Some(filter) = filter else {
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

pub(crate) struct RotationFilter {
    metadata: ReplayProtectionBloomfilterMetadata,
    data: Bloom<[u8; REPLAY_TAG_SIZE]>,
}

impl RotationFilter {
    pub(crate) fn new(
        items_count: usize,
        fp_p: f64,
        packets_received_at_creation: usize,
        rotation_id: u32,
    ) -> Result<Self, NymNodeError> {
        let filter =
            Bloom::new_for_fp_rate(items_count, fp_p).map_err(NymNodeError::bloomfilter_failure)?;

        Ok(RotationFilter {
            metadata: ReplayProtectionBloomfilterMetadata {
                creation_time: OffsetDateTime::now_utc(),
                packets_received_at_creation,
                rotation_id,
            },
            data: filter,
        })
    }

    // due to the size of the bloomfilter, extra caution has to be applied when using this method
    // note: we're not getting reference to bytes as this method is used when flushing data to the disk
    // (which takes ~30s) and we can't block the mutex for that long.
    fn bytes(&self) -> Vec<u8> {
        // attach metadata bytes at the end as it would make deserialisation cheaper (as we could avoid
        // copying the bloomfilter bytes twice)
        let mut bloom_bytes = self.data.to_bytes();
        bloom_bytes.extend_from_slice(&self.metadata.bytes());
        bloom_bytes
    }

    pub(crate) fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, NymNodeError> {
        let len = bytes.len();
        if bytes.len() < ReplayProtectionBloomfilterMetadata::SERIALIZED_LEN {
            return Err(NymNodeError::BloomfilterMetadataDeserialisationFailure);
        }

        let mut bloom_bytes = bytes;
        let metadata_bytes =
            bloom_bytes.split_off(len - ReplayProtectionBloomfilterMetadata::SERIALIZED_LEN);

        Ok(RotationFilter {
            metadata: ReplayProtectionBloomfilterMetadata::try_from_bytes(&metadata_bytes)?,
            data: Bloom::from_bytes(bloom_bytes).map_err(NymNodeError::bloomfilter_failure)?,
        })
    }

    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        info!("attempting to load prior replay detection bloomfilter...");
        let path = path.as_ref();
        let mut file = File::open(path).map_err(|source| NymNodeError::BloomfilterIoFailure {
            source,
            path: path.to_path_buf(),
        })?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|source| NymNodeError::BloomfilterIoFailure {
                source,
                path: path.to_path_buf(),
            })?;

        RotationFilter::try_from_bytes(buf)
    }
}
