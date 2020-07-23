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
use crate::kdf::blake3_hkdf;
use crate::symmetric::aes_ctr::{generic_array::typenum::Unsigned, Aes128Key, Aes128KeySize};
use rand::{CryptoRng, RngCore};

pub mod asymmetric;
pub mod hmac;
pub mod kdf;
pub mod symmetric;

// TODO: this function uses all three modules: asymmetric crypto, symmetric crypto and derives key...,
// so I don't know where to put it...

/// Generate an ephemeral encryption keypair and perform diffie-hellman to establish
/// shared key with the remote.
pub fn new_ephemeral_shared_key<R>(
    rng: &mut R,
    remote_key: &encryption::PublicKey,
) -> (encryption::KeyPair, Aes128Key)
where
    R: RngCore + CryptoRng,
{
    let ephemeral_keypair = encryption::KeyPair::new_with_rng(rng);

    // after performing diffie-hellman we don't care about the private component anymore
    let dh_result = ephemeral_keypair.private_key().diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only 16 bytes
    let okm = blake3_hkdf::extract_then_expand(None, &dh_result, None, Aes128KeySize::to_usize())
        .expect("somehow too long okm was provided");

    let derived_shared_key =
        Aes128Key::from_exact_iter(okm).expect("okm was expanded to incorrect length!");

    (ephemeral_keypair, derived_shared_key)
}
