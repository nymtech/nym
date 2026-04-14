// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{Context, bail};
use nym_crypto::asymmetric::x25519;
use nym_pemstore::load_key;
use std::path::Path;
use std::sync::Arc;

/// Loads an x25519 Noise private key from a PEM file and returns the full key pair
/// wrapped in an [`Arc`] for shared ownership.
pub(crate) fn load_noise_key<P: AsRef<Path>>(path: P) -> anyhow::Result<Arc<x25519::KeyPair>> {
    let path = path.as_ref();
    if !path.exists() {
        bail!("noise key file does not exist at: {}", path.display());
    }
    let noise_key: x25519::PrivateKey = load_key(&path).context("failed to load noise key")?;
    Ok(Arc::new(noise_key.into()))
}
