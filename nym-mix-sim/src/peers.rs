// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP peers for the simulation, generated per run.

use std::sync::Arc;

use nym_kkt::keys::KEMKeys;
use nym_kkt_ciphersuite::{Ciphersuite, mceliece};
use nym_lp::peer::LpLocalPeer;

/// A peer with a real ML-KEM keypair and a placeholder where its Classic McEliece one would be.
///
/// What it buys is startup: McEliece keygen costs seconds per peer in a debug build, so a
/// simulation of any size otherwise spends its startup generating keys no handshake will use.
pub fn random_peer_mlkem_only<R>(rng: &mut R) -> LpLocalPeer
where
    R: rand010::CryptoRng + rand010::Rng,
{
    let kem_keys = KEMKeys::new(
        placeholder_mceliece_keypair(),
        nym_kkt::key_utils::generate_keypair_mlkem(rng),
    );

    LpLocalPeer::new(
        Ciphersuite::default(),
        Arc::new(nym_kkt::key_utils::generate_lp_keypair_x25519(rng)),
    )
    .with_kem_keys(kem_keys)
}

/// A McEliece keypair made of `1`s, standing in for one nothing will use.
fn placeholder_mceliece_keypair() -> libcrux_psq::classic_mceliece::KeyPair {
    // built on the heap rather than as `Box::new([1; _])`, which would put half a megabyte on the
    // stack on its way there
    //
    // SAFETY: each is a `vec!` of exactly the length its key type converts from, and naming those
    // lengths is what makes a mismatch a compile error rather than a panic
    #[expect(clippy::unwrap_used)]
    let pk: Box<[u8; mceliece::PUBLIC_KEY_LENGTH]> = vec![1u8; mceliece::PUBLIC_KEY_LENGTH]
        .into_boxed_slice()
        .try_into()
        .unwrap();
    #[expect(clippy::unwrap_used)]
    let sk: Box<[u8; mceliece::SECRET_KEY_LENGTH]> = vec![1u8; mceliece::SECRET_KEY_LENGTH]
        .into_boxed_slice()
        .try_into()
        .unwrap();

    libcrux_psq::classic_mceliece::KeyPair {
        pk: pk.into(),
        sk: sk.into(),
    }
}
