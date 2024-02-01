// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ChunkIndex, DealingIndex, EpochId, PartialContractDealingData};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use std::collections::BTreeMap;

/// Defines the maximum size of a dealing chunk. Currently set to 2kB
pub const MAX_DEALING_CHUNK_SIZE: usize = 2048;

/// Defines the maximum size of a full dealing.
/// Currently set to 100kB (which is enough for a dealing created for 100 parties)
pub const MAX_DEALING_SIZE: usize = 102400;

pub const MAX_DEALING_CHUNKS: usize = MAX_DEALING_SIZE / MAX_DEALING_CHUNK_SIZE;

// 2 public attributes, 2 private attributes, 1 fixed for coconut credential
pub const DEFAULT_DEALINGS: usize = 2 + 2 + 1;

#[cw_serde]
pub struct DealingChunkInfo {
    pub size: usize,
}

impl DealingChunkInfo {
    pub fn new(size: usize) -> Self {
        DealingChunkInfo { size }
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
pub struct SubmittedChunk {
    pub info: DealingChunkInfo,

    // this field is updated by the contract itself to indicate when this particular chunk has been received
    pub submission_height: Option<u64>,
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
            submission_height: None,
        }
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
        self.submitted_chunks
            .values()
            .all(|c| c.submission_height.is_some())
    }

    pub fn total_size(&self) -> usize {
        self.submitted_chunks.values().map(|c| c.info.size).sum()
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

    pub submission_height: Option<u64>,
}

#[cw_serde]
pub struct DealingStatusResponse {
    pub epoch_id: EpochId,

    pub dealer: Addr,

    pub dealing_index: DealingIndex,

    pub full_dealing_submitted: bool,
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
            assert_eq!(dealing_len, chunks.iter().map(|c| c.size).sum::<usize>());

            let mut expected_last = dealing_len % chunk_size;
            if expected_last == 0 {
                expected_last = chunk_size;
            }
            assert_eq!(chunks.last().unwrap().size, expected_last);
        }
    }
}
