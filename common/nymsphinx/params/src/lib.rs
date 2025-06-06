// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::aes::Aes128;
use nym_crypto::blake3;
use nym_crypto::ctr;
use nym_crypto::Aes256GcmSiv;

type Aes128Ctr = ctr::Ctr64BE<Aes128>;

// Re-export for ease of use
pub use key_rotation::SphinxKeyRotation;
pub use packet_sizes::PacketSize;
pub use packet_types::PacketType;
pub use packet_version::PacketVersion;

pub mod key_rotation;
pub mod packet_sizes;
pub mod packet_types;
pub mod packet_version;

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
// TODO: if updated, the pem type defined in gateway\gateway-requests\src\registration\handshake\legacy_shared_key
// needs updating!
pub type GatewayIntegrityHmacAlgorithm = blake3::Hasher;

/// Encryption algorithm used for encrypting acknowledgement messages.
// TODO: if updated:
// - PacketSize::ACK_PACKET_SIZE needs to be manually updated (if nonce/iv size differs);
// this requirement will eventually go away once const generics are stabilised (and generic_array and co. start using them)
// - the pem type defined in nym\common\nymsphinx\acknowledgements\src\key needs updating!
pub type AckEncryptionAlgorithm = Aes128Ctr;

/// Legacy encryption algorithm used for end-to-end encryption of messages exchanged between clients
/// and their gateways.
// TODO: if updated, the pem type defined in gateway\gateway-requests\src\registration\handshake\legacy_shared_key
// needs updating!
pub type LegacyGatewayEncryptionAlgorithm = Aes128Ctr;

/// Encryption algorithm used for end-to-end encryption of messages exchanged between clients
/// and their gateways.
// NOTE: if updated, the pem type defined in gateway\gateway-requests\src\registration\handshake\shared_key
pub type GatewayEncryptionAlgorithm = Aes256GcmSiv;

/// Encryption algorithm used for end-to-end encryption of messages exchanged between clients that are
/// encapsulated inside sphinx packets.
pub type PacketEncryptionAlgorithm = Aes128Ctr;

/// Encryption algorithm used for end-to-end encryption of reply messages constructed using ReplySURBs.
// TODO: I don't see any reason for it to be different than what is used for regular packets. Perhaps
// it could be potentially insecure to use anything else?
pub type ReplySurbEncryptionAlgorithm = PacketEncryptionAlgorithm;
