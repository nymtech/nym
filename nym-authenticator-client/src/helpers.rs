// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::x25519::KeyPair;
use nym_pemstore::KeyPairPath;
use nym_sdk::mixnet::{IncludedSurbs, Recipient, TransmissionLane};
use rand::{CryptoRng, RngCore};

pub(crate) fn create_input_message(
    recipient: Recipient,
    data: Vec<u8>,
    surbs: IncludedSurbs,
) -> nym_sdk::mixnet::InputMessage {
    match surbs {
        IncludedSurbs::Amount(surbs) => nym_sdk::mixnet::InputMessage::new_anonymous(
            recipient,
            data,
            surbs,
            TransmissionLane::General,
            None,
        ),
        IncludedSurbs::ExposeSelfAddress => nym_sdk::mixnet::InputMessage::new_regular(
            recipient,
            data,
            TransmissionLane::General,
            None,
        ),
    }
}

pub(crate) fn load_or_generate_keypair<R: RngCore + CryptoRng>(
    rng: &mut R,
    paths: KeyPairPath,
) -> KeyPair {
    match nym_pemstore::load_keypair(&paths) {
        Ok(keypair) => keypair,
        Err(_) => {
            let keypair = KeyPair::new(rng);
            if let Err(e) = nym_pemstore::store_keypair(&keypair, &paths) {
                tracing::error!(
                    "could not store generated keypair at {:?} - {:?}; will use ephemeral keys",
                    paths,
                    e
                );
            }
            keypair
        }
    }
}
