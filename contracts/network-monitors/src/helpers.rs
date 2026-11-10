// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Shape-level validation of a base58-encoded 32-byte public key.
///
/// The key is not verified to be a valid curve point, as doing so on-chain is disproportionately
/// expensive relative to the downstream risk - a malformed key will simply fail signature
/// verification when used. The caller maps the returned reason onto its own error variant.
pub(crate) fn ensure_bs58_32_byte_key(raw: &str) -> Result<(), String> {
    let mut public_key = [0u8; 32];
    let used = bs58::decode(raw)
        .onto(&mut public_key)
        .map_err(|err| err.to_string())?;

    if used != 32 {
        return Err("Too few bytes provided for the public key".into());
    }

    Ok(())
}
