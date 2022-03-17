// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::DkgError;
use bls12_381::Scalar;
use ff::Field;
use rand_core::RngCore;
use zeroize::Zeroize;

// if this type is changed, one must ensure all values can fit in it
pub type Chunk = u16;

// note: CHUNK_BYTES * NUM_CHUNKS must equal to SCALAR_SIZE
pub const CHUNK_BYTES: usize = 2;
pub const NUM_CHUNKS: usize = 16;
pub const SCALAR_SIZE: usize = 32;

/// In paper B; number of distinct chunks
pub const CHUNK_SIZE: usize = 1 << (CHUNK_BYTES << 3);

#[derive(PartialEq, Eq, Debug, Zeroize)]
#[cfg_attr(test, derive(Clone))]
#[zeroize(drop)]
pub struct Share(pub(crate) Scalar);

impl Share {
    // not really used outside tests
    pub(crate) fn random(mut rng: impl RngCore) -> Self {
        Share(Scalar::random(&mut rng))
    }

    pub(crate) fn to_chunks(&self) -> ChunkedShare {
        let mut chunks = [0; NUM_CHUNKS];
        let mut bytes = self.0.to_bytes();

        for (chunk, chunk_bytes) in chunks.iter_mut().zip(bytes[..].chunks_exact(CHUNK_BYTES)) {
            let mut tmp = [0u8; CHUNK_BYTES];
            tmp.copy_from_slice(chunk_bytes);
            *chunk = Chunk::from_be_bytes(tmp)
        }

        bytes.zeroize();
        ChunkedShare { chunks }
    }
}

#[derive(Default, Zeroize)]
#[cfg_attr(test, derive(Clone))]
#[zeroize(drop)]
pub(crate) struct ChunkedShare {
    pub(crate) chunks: [Chunk; NUM_CHUNKS],
}

impl From<Share> for ChunkedShare {
    fn from(share: Share) -> ChunkedShare {
        share.to_chunks()
    }
}

impl TryFrom<ChunkedShare> for Share {
    type Error = DkgError;

    fn try_from(chunked: ChunkedShare) -> Result<Share, Self::Error> {
        let mut bytes = [0u8; SCALAR_SIZE];
        for (chunk, chunk_bytes) in chunked
            .chunks
            .iter()
            .zip(bytes[..].chunks_exact_mut(CHUNK_BYTES))
        {
            let tmp = chunk.to_be_bytes();
            chunk_bytes.copy_from_slice(&tmp[..]);
        }

        let recovered = Option::from(Scalar::from_bytes(&bytes))
            .map(Share)
            .ok_or(DkgError::MalformedShare)?;

        bytes.zeroize();
        Ok(recovered)
    }
}
