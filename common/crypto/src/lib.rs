// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::asymmetric::encryption;
use crate::symmetric::aes_ctr::{Aes128Key, Aes128KeySize};
pub use digest::Digest;
use digest::{BlockInput, FixedOutput, Reset, Update};
use generic_array::{typenum::Unsigned, ArrayLength};
use rand::{CryptoRng, RngCore};

pub mod asymmetric;
pub mod crypto_hash;
pub mod hkdf;
pub mod hmac;
pub mod symmetric;

// with the below my idea was to try to introduce have a single place of importing all hashing, encryption,
// etc. algorithms and import them elsewhere as needed via common/crypto
pub use blake3;

// TODO: this function uses all three modules: asymmetric crypto, symmetric crypto and derives key...,
// so I don't know where to put it...

/// Generate an ephemeral encryption keypair and perform diffie-hellman to establish
/// shared key with the remote.
// TODO: make resultant symmetric key generic (so that you could call it like new_ephemeral_shared_key::<hasher, encryption>)
pub fn new_ephemeral_shared_key<D, R>(
    rng: &mut R,
    remote_key: &encryption::PublicKey,
) -> (encryption::KeyPair, Aes128Key)
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
    R: RngCore + CryptoRng,
{
    let ephemeral_keypair = encryption::KeyPair::new_with_rng(rng);

    // after performing diffie-hellman we don't care about the private component anymore
    let dh_result = ephemeral_keypair.private_key().diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only 16 bytes
    let okm = hkdf::extract_then_expand::<D>(None, &dh_result, None, Aes128KeySize::to_usize())
        .expect("somehow too long okm was provided");

    let derived_shared_key =
        Aes128Key::from_exact_iter(okm).expect("okm was expanded to incorrect length!");

    (ephemeral_keypair, derived_shared_key)
}

pub fn recompute_shared_key<D>(
    remote_key: &encryption::PublicKey,
    local_key: &encryption::PrivateKey,
) -> Aes128Key
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    let dh_result = local_key.diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only 16 bytes
    let okm = hkdf::extract_then_expand::<D>(None, &dh_result, None, Aes128KeySize::to_usize())
        .expect("somehow too long okm was provided");

    Aes128Key::from_exact_iter(okm).expect("okm was expanded to incorrect length!")
}
