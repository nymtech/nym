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
use crypto::blake3;

pub mod message_types;
pub mod packet_sizes;

pub use message_types::MessageType;
pub use packet_sizes::PacketSize;

// If somebody can provide an argument why it might be reasonable to have more than 255 mix hops,
// I will change this to [`usize`]
pub const DEFAULT_NUM_MIX_HOPS: u8 = 3;

// TODO: ask @AP about the choice of below algorithms

/// Hashing algorithm used during hkdf for ephemeral shared key generation per sphinx packet payload.
pub type PacketHkdfAlgorithm = blake3::Hasher;

/// Hashing algorithm used during hkdf while establishing long-term shared key between client and gateway.
pub type GatewaySharedKeyHkdfAlgorithm = blake3::Hasher;

/// Hashing algorithm used when computing digest of a reply SURB encryption key.
pub type ReplySURBKeyDigestAlgorithm = blake3::Hasher;

/// Hashing algorithm used when computing integrity (H)Mac for message exchanged between client and gateway.
// TODO: if updated, the pem type defined in gateway\gateway-requests\src\registration\handshake\shared_key
// needs updating!
pub type GatewayIntegrityHmacAlgorithm = blake3::Hasher;
