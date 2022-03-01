// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::asymmetric::encryption;
use crate::hkdf::{self, HmacImpl};
use cipher::{Key, KeyIvInit, StreamCipher};
use digest::Digest;
use rand::{CryptoRng, RngCore};

/// Generate an ephemeral encryption keypair and perform diffie-hellman to establish
/// shared key with the remote.
pub fn new_ephemeral_shared_key<C, D, I, R>(
    rng: &mut R,
    remote_key: &encryption::PublicKey,
) -> (encryption::KeyPair, Key<C>)
where
    C: StreamCipher + KeyIvInit,
    D: Digest,
    I: HmacImpl<D>,
    R: RngCore + CryptoRng,
{
    let ephemeral_keypair = encryption::KeyPair::new(rng);

    // after performing diffie-hellman we don't care about the private component anymore
    let dh_result = ephemeral_keypair.private_key().diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only C::KeySize bytes
    let okm = hkdf::extract_then_expand::<D, I>(None, &dh_result, None, C::key_size())
        .expect("somehow too long okm was provided");

    let derived_shared_key =
        Key::<C>::from_exact_iter(okm).expect("okm was expanded to incorrect length!");

    (ephemeral_keypair, derived_shared_key)
}

/// Recompute shared key using remote public key and local private key.
pub fn recompute_shared_key<C, D, I>(
    remote_key: &encryption::PublicKey,
    local_key: &encryption::PrivateKey,
) -> Key<C>
where
    C: StreamCipher + KeyIvInit,
    D: Digest,
    I: HmacImpl<D>,
{
    let dh_result = local_key.diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only C::KeySize bytes
    let okm = hkdf::extract_then_expand::<D, I>(None, &dh_result, None, C::key_size())
        .expect("somehow too long okm was provided");

    Key::<C>::from_exact_iter(okm).expect("okm was expanded to incorrect length!")
}
