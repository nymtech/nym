// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_dkg::bte::PublicKeyWithProof;
use nym_dkg::Dealing;

pub(crate) fn unchecked_decode_bte_key(raw: &str) -> PublicKeyWithProof {
    let bytes = bs58::decode(raw).into_vec().unwrap();
    PublicKeyWithProof::try_from_bytes(&bytes).unwrap()
}
