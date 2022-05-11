// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::{CHUNK_BYTES, NUM_CHUNKS, SCALAR_SIZE};
use crate::error::DkgError;
use crate::interpolation::perform_lagrangian_interpolation_at_origin;
use crate::NodeIndex;
use bls12_381::Scalar;
use zeroize::Zeroize;

// if this type is changed, one must ensure all values can fit in it
pub type Chunk = u16;

#[derive(PartialEq, Eq, Debug, Zeroize)]
#[cfg_attr(test, derive(Clone))]
#[zeroize(drop)]
pub struct Share(pub(crate) Scalar);

pub fn combine_shares(shares: Vec<Share>, node_indices: &[NodeIndex]) -> Result<Scalar, DkgError> {
    if shares.len() != node_indices.len() {
        return Err(DkgError::MismatchedLagrangianSamplesLengths {
            x: node_indices.len(),
            y: shares.len(),
        });
    }

    let samples = shares
        .into_iter()
        .zip(node_indices.iter())
        .map(|(share, index)| (Scalar::from(*index), share.0))
        .collect::<Vec<_>>();

    perform_lagrangian_interpolation_at_origin(&samples)
}

impl Share {
    // not really used outside tests
    #[cfg(test)]
    pub(crate) fn random(mut rng: impl rand_core::RngCore) -> Self {
        use ff::Field;
        Share(Scalar::random(&mut rng))
    }

    pub(crate) fn to_chunks(&self) -> ChunkedShare {
        let mut chunks = [0; NUM_CHUNKS];
        let mut bytes = self.0.to_bytes();

        for (chunk, chunk_bytes) in chunks.iter_mut().zip(bytes[..].chunks_exact(CHUNK_BYTES)) {
            let mut tmp = [0u8; CHUNK_BYTES];
            tmp.copy_from_slice(chunk_bytes);
            *chunk = Chunk::from_le_bytes(tmp)
        }

        bytes.zeroize();
        ChunkedShare { chunks }
    }

    pub(crate) fn inner(&self) -> &Scalar {
        &self.0
    }
}

impl From<Scalar> for Share {
    fn from(s: Scalar) -> Self {
        Share(s)
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
            let tmp = chunk.to_le_bytes();
            chunk_bytes.copy_from_slice(&tmp[..]);
        }

        let recovered = Option::from(Scalar::from_bytes(&bytes))
            .map(Share)
            .ok_or(DkgError::MalformedShare)?;

        bytes.zeroize();
        Ok(recovered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::combine_scalar_chunks;
    use rand_core::SeedableRng;

    #[test]
    fn chunking_share() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let share = Share::random(&mut rng);
        let chunks: ChunkedShare = share.clone().into();

        let scalar_chunks = chunks
            .chunks
            .iter()
            .map(|c| Scalar::from(*c as u64))
            .collect::<Vec<_>>();
        let expected = combine_scalar_chunks(&scalar_chunks);
        assert_eq!(expected, share.0);

        let recombined: Share = chunks.try_into().unwrap();
        assert_eq!(expected, recombined.0);
    }
}
