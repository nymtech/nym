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
use crate::hkdf;
use cipher::stream::{Key, NewStreamCipher, SyncStreamCipher};
use digest::{BlockInput, FixedOutput, Reset, Update};
use generic_array::{typenum::Unsigned, ArrayLength};
use rand::{CryptoRng, RngCore};

/// Generate an ephemeral encryption keypair and perform diffie-hellman to establish
/// shared key with the remote.
pub fn new_ephemeral_shared_key<C, D, R>(
    rng: &mut R,
    remote_key: &encryption::PublicKey,
) -> (encryption::KeyPair, Key<C>)
where
    C: SyncStreamCipher + NewStreamCipher,
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
    R: RngCore + CryptoRng,
{
    let ephemeral_keypair = encryption::KeyPair::new(rng);

    // after performing diffie-hellman we don't care about the private component anymore
    let dh_result = ephemeral_keypair.private_key().diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only C::KeySize bytes
    let okm = hkdf::extract_then_expand::<D>(None, &dh_result, None, C::KeySize::to_usize())
        .expect("somehow too long okm was provided");

    let derived_shared_key =
        Key::<C>::from_exact_iter(okm).expect("okm was expanded to incorrect length!");

    (ephemeral_keypair, derived_shared_key)
}

/// Recompute shared key using remote public key and local private key.
pub fn recompute_shared_key<C, D>(
    remote_key: &encryption::PublicKey,
    local_key: &encryption::PrivateKey,
) -> Key<C>
where
    C: SyncStreamCipher + NewStreamCipher,
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    let dh_result = local_key.diffie_hellman(remote_key);

    // there is no reason for this to fail as our okm is expected to be only C::KeySize bytes
    let okm = hkdf::extract_then_expand::<D>(None, &dh_result, None, C::KeySize::to_usize())
        .expect("somehow too long okm was provided");

    Key::<C>::from_exact_iter(okm).expect("okm was expanded to incorrect length!")
}
