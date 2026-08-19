// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{Context, anyhow, bail};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::hkdf;
use nym_pemstore::load_key;
use sha2::Sha256;
use std::path::Path;
use std::sync::Arc;

/// Domain-separation label for the ed25519 client identity derived from the agent's noise key.
/// Changing it rotates every agent's identity, which then has to be re-announced on chain.
const CLIENT_IDENTITY_HKDF_LABEL: &[u8] = b"nym-network-monitor-agent-ed25519-client-identity-v1";

/// Loads an x25519 Noise private key from a PEM file and returns the full key pair
/// wrapped in an [`Arc`] for shared ownership.
pub(crate) fn load_noise_key<P: AsRef<Path>>(path: P) -> anyhow::Result<Arc<x25519::KeyPair>> {
    let path = path.as_ref();
    if !path.exists() {
        bail!("noise key file does not exist at: {}", path.display());
    }
    let noise_key: x25519::PrivateKey = load_key(path).context("failed to load noise key")?;
    Ok(Arc::new(noise_key.into()))
}

/// Derives the agent's ed25519 client identity from its x25519 noise private key, so that a gateway
/// client session needs no key material beyond the noise key the agent already holds.
///
/// The HKDF output is used directly as the ed25519 seed. The label is what keeps this derivation
/// separate from anything else that may ever be derived from the same secret - seeding a CSPRNG with
/// the raw private key would give no such separation.
pub(crate) fn derive_client_identity(
    noise_key: &x25519::KeyPair,
) -> anyhow::Result<ed25519::KeyPair> {
    let seed = hkdf::extract_then_expand::<Sha256>(
        None,
        &noise_key.private_key().to_bytes(),
        Some(CLIENT_IDENTITY_HKDF_LABEL),
        ed25519::SECRET_KEY_LENGTH,
    )
    .map_err(|err| {
        anyhow!("failed to derive the ed25519 client identity from the noise key: {err}")
    })?;

    let private_key = ed25519::PrivateKey::from_bytes(&seed)
        .context("the derived ed25519 client identity seed was not a valid private key")?;
    Ok(private_key.into())
}
