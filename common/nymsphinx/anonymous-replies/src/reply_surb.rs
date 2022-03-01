// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::encryption_key::{SurbEncryptionKey, SurbEncryptionKeyError, SurbEncryptionKeySize};
use crypto::{generic_array::typenum::Unsigned, Digest};
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, MAX_NODE_ADDRESS_UNPADDED_LEN};
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_params::{ReplySurbKeyDigestAlgorithm, DEFAULT_NUM_MIX_HOPS};
use nymsphinx_types::{delays, Error as SphinxError, SURBMaterial, SphinxPacket, SURB};
use rand::{CryptoRng, RngCore};
use serde::de::{Error as SerdeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::TryFrom;
use std::fmt::{self, Formatter};
use std::time;
use topology::{NymTopology, NymTopologyError};

#[derive(Debug)]
pub enum ReplySurbError {
    UnpaddedMessageError,
    MalformedStringError(bs58::decode::Error),
    RecoveryError(SphinxError),
    InvalidEncryptionKeyData(SurbEncryptionKeyError),
}

impl fmt::Display for ReplySurbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplySurbError::UnpaddedMessageError => {
                write!(f, "tried to use reply SURB with an unpadded message")
            }
            ReplySurbError::MalformedStringError(decode_err) => {
                write!(f, "reply SURB is incorrectly formatted: {}", decode_err)
            }
            ReplySurbError::RecoveryError(sphinx_err) => {
                write!(f, "failed to recover reply SURB from bytes: {}", sphinx_err)
            }
            ReplySurbError::InvalidEncryptionKeyData(surb_key_err) => write!(
                f,
                "failed to recover reply SURB encryption key from bytes: {}",
                surb_key_err
            ),
        }
    }
}

// since we have Debug and Display might as well slap Error on top of it too
impl std::error::Error for ReplySurbError {}

impl From<SurbEncryptionKeyError> for ReplySurbError {
    fn from(err: SurbEncryptionKeyError) -> Self {
        ReplySurbError::InvalidEncryptionKeyData(err)
    }
}

#[derive(Debug)]
pub struct ReplySurb {
    surb: SURB,
    encryption_key: SurbEncryptionKey,
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

        impl<'de> Visitor<'de> for ReplySurbVisitor {
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
        average_delay: time::Duration,
        topology: &NymTopology,
    ) -> Result<Self, NymTopologyError>
    where
        R: RngCore + CryptoRng,
    {
        let route =
            topology.random_route_to_gateway(rng, DEFAULT_NUM_MIX_HOPS, recipient.gateway())?;
        let delays = delays::generate_from_average_duration(route.len(), average_delay);
        let destination = recipient.as_sphinx_destination();

        let surb_material = SURBMaterial::new(route, delays, destination);

        // this can't fail as we know we have a valid route to gateway and have correct number of delays
        Ok(ReplySurb {
            surb: surb_material.construct_SURB().unwrap(),
            encryption_key: SurbEncryptionKey::new(rng),
        })
    }

    /// Returns the expected number of bytes the [`ReplySURB`] will take after serialization.
    /// Useful for deserialization from a bytes stream.
    pub fn serialized_len(mix_hops: u8) -> usize {
        use nymsphinx_types::{HEADER_SIZE, NODE_ADDRESS_LENGTH, PAYLOAD_KEY_SIZE};

        // the SURB itself consists of SURB_header, first hop address and set of payload keys
        // (note extra 1 for the gateway)
        SurbEncryptionKeySize::USIZE
            + HEADER_SIZE
            + NODE_ADDRESS_LENGTH
            + (1 + mix_hops as usize) * PAYLOAD_KEY_SIZE
    }

    pub fn encryption_key(&self) -> &SurbEncryptionKey {
        &self.encryption_key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // KEY || SURB_BYTES
        self.encryption_key
            .to_bytes()
            .into_iter()
            .chain(self.surb.to_bytes().into_iter())
            .collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplySurbError> {
        let encryption_key =
            SurbEncryptionKey::try_from_bytes(&bytes[..SurbEncryptionKeySize::USIZE])?;

        let surb = match SURB::from_bytes(&bytes[SurbEncryptionKeySize::USIZE..]) {
            Err(err) => return Err(ReplySurbError::RecoveryError(err)),
            Ok(surb) => surb,
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
    pub fn apply_surb(
        self,
        message: &[u8],
        packet_size: Option<PacketSize>,
    ) -> Result<(SphinxPacket, NymNodeRoutingAddress), ReplySurbError> {
        let packet_size = packet_size.unwrap_or_default();

        if message.len() != packet_size.plaintext_size() {
            return Err(ReplySurbError::UnpaddedMessageError);
        }

        // this can realistically only fail on too long messages and we just checked for that
        let (packet, first_hop) = self
            .surb
            .use_surb(message, packet_size.payload_size())
            .expect("this error indicates inconsistent message length checking - it shouldn't have happened!");

        let first_hop_address = NymNodeRoutingAddress::try_from(first_hop).unwrap();

        Ok((packet, first_hop_address))
    }
}
