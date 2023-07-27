// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::LedgerError;
use crate::helpers::answer_bytes;
use bip32::{PublicKey, PublicKeyBytes};
use ledger_transport::APDUAnswer;

/// SECP256K1 address of the device.
pub struct AddrSecp256k1Response {
    /// SECP256K1 public key.
    pub public_key: k256::PublicKey,
    /// String representation of the Cosmos address.
    pub address: String,
}

impl TryFrom<APDUAnswer<Vec<u8>>> for AddrSecp256k1Response {
    type Error = LedgerError;

    fn try_from(answer: APDUAnswer<Vec<u8>>) -> Result<Self, Self::Error> {
        let bytes = answer_bytes(&answer)?;
        if bytes.len() < 33 {
            return Err(Self::Error::InvalidAnswerLength {
                expected: 33,
                received: bytes.len(),
            });
        }

        let (pub_key, addr) = bytes.split_at(33);
        let public_key = PublicKey::from_bytes(
            PublicKeyBytes::try_from(pub_key).expect("Public key should be 33 bytes"),
        )?;
        let address = String::from_utf8(addr.to_vec()).unwrap();

        Ok(AddrSecp256k1Response {
            public_key,
            address,
        })
    }
}
