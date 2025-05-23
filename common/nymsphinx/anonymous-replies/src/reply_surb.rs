// Copyright 2021-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::encryption_key::{SurbEncryptionKey, SurbEncryptionKeyError, SurbEncryptionKeySize};
use nym_crypto::{generic_array::typenum::Unsigned, Digest};
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::{
    NymNodeRoutingAddress, NymNodeRoutingAddressError, MAX_NODE_ADDRESS_UNPADDED_LEN,
};
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::{PacketType, ReplySurbKeyDigestAlgorithm};
use nym_sphinx_types::{
    NymPacket, SURBMaterial, SphinxError, HEADER_SIZE, NODE_ADDRESS_LENGTH, SURB,
    X25519_WITH_EXPLICIT_PAYLOAD_KEYS_VERSION,
};
use nym_topology::{NymRouteProvider, NymTopologyError};
use rand::{CryptoRng, RngCore};
use serde::de::{Error as SerdeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Formatter};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplySurbError {
    #[error("did not receive enough data to recover a reply SURB")]
    TooShort,

    #[error("tried to use reply SURB with an unpadded message")]
    UnpaddedMessageError,

    #[error("reply SURB is incorrectly formatted: {0}")]
    MalformedStringError(#[from] bs58::decode::Error),

    #[error("failed to recover reply SURB from bytes: {0}")]
    RecoveryError(#[from] SphinxError),

    #[error("failed to validate the first hop address of the recovered reply SURB: {0}")]
    MalformedSurbFirstHop(#[from] NymNodeRoutingAddressError),

    #[error("failed to recover reply SURB encryption key from bytes: {0}")]
    InvalidEncryptionKeyData(#[from] SurbEncryptionKeyError),
}

#[derive(Debug)]
pub struct ReplySurb {
    pub(crate) surb: SURB,
    pub(crate) encryption_key: SurbEncryptionKey,
}

// Serialize + Deserialize is not really used anymore (it was for a CBOR experiment)
// however, if we decided we needed it again, it's already here
impl Serialize for ReplySurb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.to_bytes())
    }
}

impl<'de> Deserialize<'de> for ReplySurb {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct ReplySurbVisitor;

        impl Visitor<'_> for ReplySurbVisitor {
            type Value = ReplySurb;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write!(formatter, "A replySURB must contain a valid symmetric encryption key and a correctly formed sphinx header")
            }

            fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                ReplySurb::from_bytes(bytes)
                    .map_err(|_| SerdeError::invalid_length(bytes.len(), &self))
            }
        }

        deserializer.deserialize_bytes(ReplySurbVisitor)
    }
}

impl ReplySurb {
    /// base overhead of a reply surb that exists regardless of type or number of key materials.
    pub(crate) const BASE_OVERHEAD: usize =
        SurbEncryptionKeySize::USIZE + HEADER_SIZE + NODE_ADDRESS_LENGTH;

    pub fn max_msg_len(packet_size: PacketSize) -> usize {
        // For detailed explanation (of ack overhead) refer to common\nymsphinx\src\preparer.rs::available_plaintext_per_packet()
        let ack_overhead = MAX_NODE_ADDRESS_UNPADDED_LEN + PacketSize::AckPacket.size();
        packet_size.plaintext_size() - ack_overhead - ReplySurbKeyDigestAlgorithm::output_size() - 1
    }

    // TODO: should this return `ReplySURBError` for consistency sake
    // or keep `NymTopologyError` because it's the only error it can actually return?
    pub fn construct<R>(
        rng: &mut R,
        recipient: &Recipient,
        average_delay: Duration,
        use_legacy_surb_format: bool,
        topology: &NymRouteProvider,
        _disable_mix_hops: bool, // TODO: support SURBs with no mix hops after changes to surb format / construction
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
    {
        let route = topology.random_route_to_egress(rng, recipient.gateway())?;
        let delays = nym_sphinx_routing::generate_hop_delays(average_delay, route.len());
        let destination = recipient.as_sphinx_destination();

        let mut surb_material = SURBMaterial::new(route, delays, destination);
        if use_legacy_surb_format {
            surb_material = surb_material.with_version(X25519_WITH_EXPLICIT_PAYLOAD_KEYS_VERSION)
        }

        // this can't fail as we know we have a valid route to gateway and have correct number of delays
        Ok(ReplySurb {
            surb: surb_material.construct_SURB().unwrap(),
            encryption_key: SurbEncryptionKey::new(rng),
        })
    }

    pub fn encryption_key(&self) -> &SurbEncryptionKey {
        &self.encryption_key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // KEY || SURB_BYTES
        self.encryption_key
            .to_bytes()
            .into_iter()
            .chain(self.surb.to_bytes())
            .collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplySurbError> {
        if bytes.len() <= SurbEncryptionKeySize::USIZE {
            return Err(ReplySurbError::TooShort);
        }

        let encryption_key =
            SurbEncryptionKey::try_from_bytes(&bytes[..SurbEncryptionKeySize::USIZE])?;

        let surb = match SURB::from_bytes(&bytes[SurbEncryptionKeySize::USIZE..]) {
            Err(err) => return Err(ReplySurbError::RecoveryError(err)),
            Ok(surb) => {
                // we can't really check fully validity of the header, but at the very least we could make a sanity check
                // to make sure the first hop address is a valid socket address
                let _ = NymNodeRoutingAddress::try_from(surb.first_hop())?;
                surb
            }
        };

        Ok(ReplySurb {
            surb,
            encryption_key,
        })
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<S: Into<String>>(val: S) -> Result<Self, ReplySurbError> {
        let bytes = match bs58::decode(val.into()).into_vec() {
            Ok(decoded) => decoded,
            Err(err) => return Err(ReplySurbError::MalformedStringError(err)),
        };
        Self::from_bytes(&bytes)
    }

    // Allows to optionally increase the packet size to send slightly longer reply.
    // the "used" surb produces the following bytes:
    // note that the `message` argument is expected to already contain all the required parts, i.e.:
    // - surb-ack
    // - key digest
    // - encrypted plaintext with padding to constant length
    pub fn apply_surb<M: AsRef<[u8]>>(
        self,
        message: M,
        packet_size: PacketSize,
        _packet_type: PacketType,
    ) -> Result<(NymPacket, NymNodeRoutingAddress), ReplySurbError> {
        let message_bytes = message.as_ref();
        if message_bytes.len() != packet_size.plaintext_size() {
            return Err(ReplySurbError::UnpaddedMessageError);
        }

        // this can realistically only fail on too long messages and we just checked for that
        let (packet, first_hop) = self
            .surb
            .use_surb(message_bytes, packet_size.payload_size())
            .expect("this error indicates inconsistent message length checking - it shouldn't have happened!");

        let first_hop_address = NymNodeRoutingAddress::try_from(first_hop).unwrap();

        Ok((NymPacket::Sphinx(packet), first_hop_address))
    }
}
