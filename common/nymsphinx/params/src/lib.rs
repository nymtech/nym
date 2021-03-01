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
use crypto::aes_ctr::Aes128Ctr;
use crypto::blake3;

// Re-export for ease of use
pub use packet_modes::PacketMode;
pub use packet_sizes::PacketSize;

pub mod packet_modes;
pub mod packet_sizes;

// If somebody can provide an argument why it might be reasonable to have more than 255 mix hops,
// I will change this to [`usize`]
pub const DEFAULT_NUM_MIX_HOPS: u8 = 3;

// TODO: not entirely sure how to feel about those being defined here, ideally it'd be where [`Fragment`]
// is defined, but that'd introduce circular dependencies as the acknowledgements crate also needs
// access to that
pub const FRAG_ID_LEN: usize = 5;
pub type SerializedFragmentIdentifier = [u8; FRAG_ID_LEN];

// TODO: ask @AP about the choice of below algorithms

/// Hashing algorithm used during hkdf for ephemeral shared key generation per sphinx packet payload.
pub type PacketHkdfAlgorithm = blake3::Hasher;

/// Hashing algorithm used during hkdf while establishing long-term shared key between client and gateway.
pub type GatewaySharedKeyHkdfAlgorithm = blake3::Hasher;

/// Hashing algorithm used when computing digest of a reply SURB encryption key.
pub type ReplySurbKeyDigestAlgorithm = blake3::Hasher;

/// Hashing algorithm used when computing integrity (H)Mac for message exchanged between client and gateway.
// TODO: if updated, the pem type defined in gateway\gateway-requests\src\registration\handshake\shared_key
// needs updating!
pub type GatewayIntegrityHmacAlgorithm = blake3::Hasher;

/// Encryption algorithm used for encrypting acknowledgement messages.
// TODO: if updated:
// - PacketSize::ACK_PACKET_SIZE needs to be manually updated (if nonce/iv size differs);
// this requirement will eventually go away once const generics are stabilised (and generic_array and co. start using them)
// - the pem type defined in nym\common\nymsphinx\acknowledgements\src\key needs updating!
pub type AckEncryptionAlgorithm = Aes128Ctr;

/// Encryption algorithm used for end-to-end encryption of messages exchanged between clients
/// and their gateways.
// TODO: if updated, the pem type defined in gateway\gateway-requests\src\registration\handshake\shared_key
// needs updating!
pub type GatewayEncryptionAlgorithm = Aes128Ctr;

/// Encryption algorithm used for end-to-end encryption of messages exchanged between clients that are
/// encapsulated inside sphinx packets.
pub type PacketEncryptionAlgorithm = Aes128Ctr;

/// Encryption algorithm used for end-to-end encryption of reply messages constructed using ReplySURBs.
// TODO: I don't see any reason for it to be different than what is used for regular packets. Perhaps
// it could be potentially insecure to use anything else?
pub type ReplySurbEncryptionAlgorithm = PacketEncryptionAlgorithm;
