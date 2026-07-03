// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Trust anchors: produce a directory digest the caller is willing to trust at a height.

use crate::error::DirectoryClientError;
use async_trait::async_trait;
use nym_lthash::LtHash16;
use nym_validator_client::nyxd::Height;

pub mod attested;
pub mod proven;

/// abci path for a raw wasm-store read.
const WASM_STORE_PATH: &str = "/store/wasm/key";

/// A directory digest trusted at a specific height.
pub struct TrustedDigest {
    pub height: Height,
    pub accumulator: LtHash16,
}

/// Produces the directory digest a caller is willing to trust at a height. The
/// verification core (fetch all entries at `H`, recompute, compare) is independent of
/// which anchor produced the digest, so alternative anchors (a nym-api quorum, a full
/// light client) can replace the proven one without touching the verifier.
#[async_trait]
pub trait DirectoryTrustAnchor {
    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError>;
}
