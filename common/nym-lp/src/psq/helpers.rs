// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_psq::handshake::ciphersuites::CiphersuiteName;
use nym_kkt_ciphersuite::KEM;

pub(crate) fn kem_to_ciphersuite(kem: KEM) -> CiphersuiteName {
    match kem {
        KEM::MlKem768 => CiphersuiteName::X25519_MLKEM768_X25519_AESGCM128_HKDFSHA256,
        KEM::McEliece => CiphersuiteName::X25519_CLASSICMCELIECE_X25519_AESGCM128_HKDFSHA256,
    }
}
