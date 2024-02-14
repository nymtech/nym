// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ChunkIndex, DealingIndex, EpochId, PartialContractDealingData};
use contracts_common::dealings::ContractSafeBytes;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use std::collections::{BTreeMap, HashMap};

/// Defines the maximum size of a dealing chunk. Currently set to 2kB
pub const MAX_DEALING_CHUNK_SIZE: usize = 2048;

/// Defines the maximum size of a full dealing.
/// Currently set to 100kB (which is enough for a dealing created for 100 parties)
pub const MAX_DEALING_SIZE: usize = 102400;

pub const MAX_DEALING_CHUNKS: usize = MAX_DEALING_SIZE / MAX_DEALING_CHUNK_SIZE;

// 2 public attributes, 2 private attributes, 1 fixed for coconut credential
pub const DEFAULT_DEALINGS: usize = 2 + 2 + 1;

pub fn chunk_dealing(
    dealing_index: DealingIndex,
    dealing_bytes: Vec<u8>,
    chunk_size: usize,
) -> HashMap<ChunkIndex, PartialContractDealing> {
    let mut chunks = HashMap::new();
    for (chunk_index, chunk) in dealing_bytes.chunks(chunk_size).enumerate() {
        let chunk = PartialContractDealing {
            dealing_index,
            chunk_index: chunk_index as ChunkIndex,
            data: ContractSafeBytes(chunk.to_vec()),
        };
        chunks.insert(chunk_index as ChunkIndex, chunk);
    }

    chunks
}

#[cw_serde]
#[derive(Copy)]
pub struct DealingChunkInfo {
    pub size: u64,
}

impl DealingChunkInfo {
    pub fn new(size: usize) -> Self {
        DealingChunkInfo { size: size as u64 }
    }

    pub fn construct(dealing_len: usize, chunk_size: usize) -> Vec<Self> {
        let (full_chunks, overflow) = (dealing_len / chunk_size, dealing_len % chunk_size);

        let mut chunks = Vec::new();
        for _ in 0..full_chunks {
            chunks.push(DealingChunkInfo::new(chunk_size));
        }

        if overflow != 0 {
            chunks.push(DealingChunkInfo::new(overflow));
        }

        chunks
    }
}

#[cw_serde]
#[derive(Copy)]
pub struct SubmittedChunk {
    pub info: DealingChunkInfo,

    pub status: ChunkSubmissionStatus,
}

#[cw_serde]
#[derive(Default, Copy)]
pub struct ChunkSubmissionStatus {
    // this field is updated by the contract itself to indicate when this particular chunk has been received
    pub submission_height: Option<u64>,
}

impl ChunkSubmissionStatus {
    pub fn submitted(&self) -> bool {
        self.submission_height.is_some()
    }
}

impl From<DealingChunkInfo> for SubmittedChunk {
    fn from(value: DealingChunkInfo) -> Self {
        SubmittedChunk::new(value)
    }
}

impl SubmittedChunk {
    pub fn new(info: DealingChunkInfo) -> Self {
        SubmittedChunk {
            info,
            status: Default::default(),
        }
    }

    pub fn submitted(&self) -> bool {
        self.status.submitted()
    }
}

#[cw_serde]
pub struct DealingMetadata {
    pub dealing_index: DealingIndex,

    pub submitted_chunks: BTreeMap<ChunkIndex, SubmittedChunk>,
}

impl DealingMetadata {
    pub fn new(dealing_index: DealingIndex, chunks: Vec<DealingChunkInfo>) -> Self {
        DealingMetadata {
            dealing_index,
            submitted_chunks: chunks
                .into_iter()
                .enumerate()
                .map(|(id, chunk)| (id as ChunkIndex, chunk.into()))
                .collect(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.submitted_chunks.values().all(|c| c.submitted())
    }

    pub fn total_size(&self) -> usize {
        self.submitted_chunks
            .values()
            .map(|c| c.info.size as usize)
            .sum()
    }

    pub fn submission_statuses(&self) -> BTreeMap<ChunkIndex, ChunkSubmissionStatus> {
        self.submitted_chunks
            .iter()
            .map(|(id, c)| (*id, c.status))
            .collect()
    }
}

#[cw_serde]
pub struct PartialContractDealing {
    pub dealing_index: DealingIndex,
    pub chunk_index: ChunkIndex,
    pub data: PartialContractDealingData,
}

impl PartialContractDealing {
    pub fn new(
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
        data: PartialContractDealingData,
    ) -> Self {
        PartialContractDealing {
            dealing_index,
            chunk_index,
            data,
        }
    }
}

#[cw_serde]
pub struct DealingMetadataResponse {
    pub epoch_id: EpochId,

    pub dealer: Addr,

    pub dealing_index: DealingIndex,

    pub metadata: Option<DealingMetadata>,
}

#[cw_serde]
pub struct DealingChunkResponse {
    pub epoch_id: EpochId,

    pub dealer: Addr,

    pub dealing_index: DealingIndex,

    pub chunk_index: ChunkIndex,

    pub chunk: Option<PartialContractDealingData>,
}

#[cw_serde]
pub struct DealingChunkStatusResponse {
    pub epoch_id: EpochId,

    pub dealer: Addr,

    pub dealing_index: DealingIndex,

    pub chunk_index: ChunkIndex,

    pub status: ChunkSubmissionStatus,
}

#[cw_serde]
pub struct DealingStatusResponse {
    pub epoch_id: EpochId,

    pub dealer: Addr,

    pub dealing_index: DealingIndex,

    pub status: DealingStatus,
}

#[cw_serde]
pub struct DealingStatus {
    pub has_metadata: bool,

    pub fully_submitted: bool,

    pub chunk_submission_status: BTreeMap<ChunkIndex, ChunkSubmissionStatus>,
}

impl From<Option<DealingMetadata>> for DealingStatus {
    fn from(metadata: Option<DealingMetadata>) -> Self {
        DealingStatus {
            has_metadata: metadata.is_some(),
            fully_submitted: metadata
                .as_ref()
                .map(|m| m.is_complete())
                .unwrap_or_default(),
            chunk_submission_status: metadata
                .map(|m| m.submission_statuses())
                .unwrap_or_default(),
        }
    }
}

#[cw_serde]
pub struct DealerDealingsStatusResponse {
    pub epoch_id: EpochId,

    pub dealer: Addr,

    pub all_dealings_fully_submitted: bool,

    pub dealing_submission_status: BTreeMap<DealingIndex, DealingStatus>,
}

impl DealerDealingsStatusResponse {
    pub fn full_dealings(&self) -> usize {
        self.dealing_submission_status
            .values()
            .filter(|s| s.fully_submitted)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_dealings() {
        const CHUNK_SIZE: usize = 512;

        let test_cases = [
            (CHUNK_SIZE - 10, CHUNK_SIZE, 1),
            (CHUNK_SIZE, CHUNK_SIZE, 1),
            (CHUNK_SIZE + 10, CHUNK_SIZE, 2),
            (CHUNK_SIZE * 2, CHUNK_SIZE, 2),
            (CHUNK_SIZE * 2 + 1, CHUNK_SIZE, 3),
            (CHUNK_SIZE * 10 + 42, CHUNK_SIZE, 11),
        ];

        for (dealing_len, chunk_size, expected_chunks) in test_cases {
            let chunks = DealingChunkInfo::construct(dealing_len, chunk_size);
            assert_eq!(expected_chunks, chunks.len());
            assert_eq!(
                dealing_len as u64,
                chunks.iter().map(|c| c.size).sum::<u64>()
            );

            let mut expected_last = dealing_len % chunk_size;
            if expected_last == 0 {
                expected_last = chunk_size;
            }
            assert_eq!(chunks.last().unwrap().size, expected_last as u64);
        }
    }
}
