// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// unfortunately we have to redefine cosmwasm' MockApi,
// as in cw1.0 they have set `CANONICAL_LENGTH` to 54 which makes
// `addr_validate` of our existing contracts fail (say of 'n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw')
// this has changed in 1.2 but we can't use that version yet...

use cosmwasm_std::testing::{digit_sum, riffle_shuffle, MockApi};
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};

const CANONICAL_LENGTH: usize = 90; // n = 45
const SHUFFLES_ENCODE: usize = 10;
const SHUFFLES_DECODE: usize = 2;

#[derive(Copy, Clone)]
pub(crate) struct CW12MockApi {
    inner: MockApi,
    canonical_length: usize,
}

impl Default for CW12MockApi {
    fn default() -> Self {
        CW12MockApi {
            inner: MockApi::default(),
            canonical_length: CANONICAL_LENGTH,
        }
    }
}

// whatever we can, hand over to 1.0 MockApi
impl Api for CW12MockApi {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        let canonical = self.addr_canonicalize(input)?;
        let normalized = self.addr_humanize(&canonical)?;
        if input != normalized {
            return Err(StdError::generic_err(
                "Invalid input: address not normalized",
            ));
        }

        Ok(Addr::unchecked(input))
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        // Dummy input validation. This is more sophisticated for formats like bech32, where format and checksum are validated.
        let min_length = 3;
        let max_length = self.canonical_length;
        if input.len() < min_length {
            return Err(StdError::generic_err(
                format!("Invalid input: human address too short for this mock implementation (must be >= {min_length})."),
            ));
        }
        if input.len() > max_length {
            return Err(StdError::generic_err(
                format!("Invalid input: human address too long for this mock implementation (must be <= {max_length})."),
            ));
        }

        // mimicks formats like hex or bech32 where different casings are valid for one address
        let normalized = input.to_lowercase();

        let mut out = Vec::from(normalized);

        // pad to canonical length with NULL bytes
        out.resize(self.canonical_length, 0x00);
        // content-dependent rotate followed by shuffle to destroy
        // the most obvious structure (https://github.com/CosmWasm/cosmwasm/issues/552)
        let rotate_by = digit_sum(&out) % self.canonical_length;
        out.rotate_left(rotate_by);
        for _ in 0..SHUFFLES_ENCODE {
            out = riffle_shuffle(&out);
        }
        Ok(out.into())
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        if canonical.len() != self.canonical_length {
            return Err(StdError::generic_err(
                "Invalid input: canonical address length not correct",
            ));
        }

        let mut tmp: Vec<u8> = canonical.clone().into();
        // Shuffle two more times which restored the original value (24 elements are back to original after 20 rounds)
        for _ in 0..SHUFFLES_DECODE {
            tmp = riffle_shuffle(&tmp);
        }
        // Rotate back
        let rotate_by = digit_sum(&tmp) % self.canonical_length;
        tmp.rotate_right(rotate_by);
        // Remove NULL bytes (i.e. the padding)
        let trimmed = tmp.into_iter().filter(|&x| x != 0x00).collect();
        // decode UTF-8 bytes into string
        let human = String::from_utf8(trimmed)?;
        Ok(Addr::unchecked(human))
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.inner
            .secp256k1_verify(message_hash, signature, public_key)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.inner
            .secp256k1_recover_pubkey(message_hash, signature, recovery_param)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.inner.ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        self.inner
            .ed25519_batch_verify(messages, signatures, public_keys)
    }

    fn debug(&self, message: &str) {
        self.inner.debug(message)
    }
}
