//! # The `outfox` format
//!
//! We define a simple mix packet format geared towards simplicity and performance with the features that are
//! specifically required by a stratified mix topology.
//!
//! `Outfox` assumes that all
//! paths are the same length (no need to hide path lengths), mixes are arranged in layers and therefore
//! know their position in a message path (no need to hide this position). These assumptions allow us
//! to do away with some of the padding traditionally used; further we prioritize efficient computation
//! over very low-bandwidth, as it seems the rate of public key operations is a bottleneck for mixes
//! rather than the availablility of bandwidth.
//!
//! ## Overview and Parameters
//!
//! In a mix network with a stratified topology packets are mixed by nodes at each of the layers. Each layer
//! 'strips' the packet from one layer of encryption, recovers the address of the mix at the next layer, and
//! passes the decoded packet to them. An identifier per processed message is stored and checked to prevent
//! replays of processed messages at each layer. Additional measures, such as adding delays, adding dummy
//! traffic or dropping messages can be empued at each mix to frustrate traffic analysis.
//!
//!
//! A layer of mix processing is defined by three parameters, included in the structure [MixStageParameters]:
//! * The `routing_information_length_bytes` (`R`) states the number of bytes representing
//!   routing information at this layer.
//! * The `remaining_header_length_bytes` (`H`) represents the remaining bytes of the packet header.
//! * The `payload_length_bytes` (`P`).
//!
//! In addition we define two system-wide constants, namely `GROUPELEMENTBYTES` (`GE`=32) and
//! `TAGBYTES` (`T`=24).
//!
//! ## Packet format, decoding
//!
//! A mix at this layer takes in messages of length `GE+T+R+H+P`, and outputs messages of length `H+P`.
//!
//! An input message is processed as follows:
//!
//! * The input packet is parsed as a `[Pk, Tag, Header, Payload]` of length `[GE, T, R+H, P]` respectivelly.
//! * A master key is derived by performing scalar multiplication with the mix secret 's', ie `K = s * Pk`.
//!   The master key is stored and checked for duplicates (if it is found processing ends.)
//! * The master key is used to perform AEAD decryption of the `Header` with an IV of zeros and the `tag`. If
//!   decryption fails processing ends. Otherwise the Header is parsed as `[Routing, Next_Header]` of length
//!   `[R, H]` respectivelly. The routing data `Routing` can be used by the mix to dertermine the next mix.
//! * Finally, the master key is used to perform lion decoding of the `Payload` into `Next_Payload`.
//! * The output packet for the next mix is `[Next_Header, Next_Payload]`.
//!
//! As an AEAD we use `chacha20poly1305_ietf` and for public key operations we use `curve25519`.
//!
//! ## Packet encoding
//!
//! Encoding is
//! performed layer by layer starting with the last hop on the route, and ending with the first. At each stage
//! of encoding a new Secret key `Sk` and corresponding `Pk` is chosen. The layer master key for the layer is
//! derived using the mix public key. And the master key is used to AEAD encrypt the concatenation of the
//! routing data for the layer, and the remaining Header; separately the master key is used to lion encrypt
//! the payload. The process is repeated for each layer (from last to first) to construct the full message.

use chacha20poly1305::AeadInPlace;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::KeyInit;

use chacha20poly1305::Tag;
use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;
use serde::Deserialize;
use serde::Serialize;
use sphinx_packet::route::Node;

use std::convert::TryInto;

const GROUPELEMENTBYTES: usize = 32;
const TAGBYTES: usize = 16;

use std::ops::Range;
use std::u8;

use crate::error::OutfoxError;
use crate::lion::*;

/// A structure that holds mix packet construction parameters. These incluse the length
/// of the routing information at each hop, the number of hops, and the payload length.
#[derive(Serialize, Deserialize)]
pub struct MixCreationParameters {
    /// The routing length is inner first, so \[0\] is the innermost routing length, etc (in bytes)
    pub routing_information_length_by_stage: Vec<usize>,
    /// The payload length (in bytes)
    pub payload_length_bytes: usize,
}

impl MixCreationParameters {
    /// Create a set of parameters for a mix packet format.
    pub fn new(payload_length_bytes: usize) -> MixCreationParameters {
        MixCreationParameters {
            routing_information_length_by_stage: Vec::new(),
            payload_length_bytes,
        }
    }

    /// Add another outer layer containing some byte length of routing data.
    pub fn add_outer_layer(&mut self, routing_information_length_bytes: usize) {
        self.routing_information_length_by_stage
            .push(routing_information_length_bytes);
    }

    /// The length of the buffer needed to build a packet.
    pub fn total_packet_length(&self) -> usize {
        let mut len = self.payload_length_bytes;
        for stage_len in &self.routing_information_length_by_stage {
            len += stage_len + GROUPELEMENTBYTES + TAGBYTES
        }
        len
    }

    /// Get the mix packet parameters for a single stage of mixing.
    pub fn get_stage_params(&self, layer_number: usize) -> (Range<usize>, MixStageParameters) {
        assert!(layer_number < self.routing_information_length_by_stage.len());

        let mut remaining_header_length_bytes = 0;
        for (i, stage_len) in self.routing_information_length_by_stage.iter().enumerate() {
            if i == layer_number {
                let params = MixStageParameters {
                    routing_information_length_bytes: *stage_len,
                    remaining_header_length_bytes,
                    payload_length_bytes: self.payload_length_bytes,
                };

                let total_size = self.total_packet_length();
                let inner_size = params.incoming_packet_length();

                return (total_size - inner_size..total_size, params);
            } else {
                remaining_header_length_bytes += stage_len + GROUPELEMENTBYTES + TAGBYTES;
            }
        }

        unreachable!();
    }
}

/// A structure representing the parameters of a single stage of mixing.
pub struct MixStageParameters {
    /// The routing information length for this stage of mixing
    pub routing_information_length_bytes: usize,
    /// The reamining header length for this stage of mixing
    pub remaining_header_length_bytes: usize,
    /// The payload length
    pub payload_length_bytes: usize,
}

impl MixStageParameters {
    pub fn incoming_packet_length(&self) -> usize {
        GROUPELEMENTBYTES + TAGBYTES + self.outgoing_packet_length()
    }

    pub fn outgoing_packet_length(&self) -> usize {
        self.routing_information_length_bytes
            + self.remaining_header_length_bytes
            + self.payload_length_bytes
    }

    pub fn pub_element_range(&self) -> Range<usize> {
        0..GROUPELEMENTBYTES
    }

    pub fn tag_range(&self) -> Range<usize> {
        GROUPELEMENTBYTES..GROUPELEMENTBYTES + TAGBYTES
    }

    pub fn routing_data_range(&self) -> Range<usize> {
        GROUPELEMENTBYTES + TAGBYTES
            ..GROUPELEMENTBYTES + TAGBYTES + self.routing_information_length_bytes
    }

    pub fn header_range(&self) -> Range<usize> {
        GROUPELEMENTBYTES + TAGBYTES
            ..GROUPELEMENTBYTES
                + TAGBYTES
                + self.routing_information_length_bytes
                + self.remaining_header_length_bytes
    }

    pub fn payload_range(&self) -> Range<usize> {
        self.incoming_packet_length() - self.payload_length_bytes..self.incoming_packet_length()
    }

    pub fn encode_mix_layer(
        &self,
        buffer: &mut [u8],
        user_secret_key: &[u8],
        node: &Node,
    ) -> Result<MontgomeryPoint, OutfoxError> {
        let routing_data = node.address.as_bytes().to_vec();
        let mix_public_key = MontgomeryPoint(*node.pub_key.as_bytes());
        let user_secret_key = Scalar::from_bytes_mod_order(user_secret_key.try_into()?);

        if buffer.len() != self.incoming_packet_length() {
            return Err(OutfoxError::LenMismatch {
                expected: buffer.len(),
                got: self.incoming_packet_length(),
            });
        }

        if routing_data.len() != self.routing_information_length_bytes {
            return Err(OutfoxError::LenMismatch {
                expected: routing_data.len(),
                got: self.routing_information_length_bytes,
            });
        }

        let user_public_key = (&ED25519_BASEPOINT_TABLE * &user_secret_key).to_montgomery();
        let shared_key = user_secret_key * mix_public_key;

        // Copy rounting data into buffer
        buffer[self.routing_data_range()].copy_from_slice(&routing_data);

        // Perform the AEAD
        let header_aead_key = ChaCha20Poly1305::new_from_slice(&shared_key.0[..])?;
        let nonce = [0u8; 12];

        let tag = header_aead_key
            .encrypt_in_place_detached(&nonce.into(), &[], &mut buffer[self.header_range()])
            .map_err(|e| OutfoxError::ChaCha20Poly1305Error(e.to_string()))?;

        // Copy Tag into buffer
        buffer[self.tag_range()].copy_from_slice(&tag[..]);

        // Copy own public key into buffer
        buffer[self.pub_element_range()].copy_from_slice(&user_public_key.0[..]);

        // Do a round of LION on the payload
        lion_transform_encrypt(&mut buffer[self.payload_range()], &shared_key.0)?;

        Ok(shared_key)
    }

    pub fn decode_mix_layer(
        &self,
        buffer: &mut [u8],
        mix_secret_key: &[u8],
    ) -> Result<MontgomeryPoint, OutfoxError> {
        // Check the length of the incoming buffer is correct.

        let mix_secret_key = Scalar::from_bytes_mod_order(mix_secret_key.try_into()?);

        if buffer.len() != self.incoming_packet_length() {
            return Err(OutfoxError::LenMismatch {
                expected: buffer.len(),
                got: self.incoming_packet_length(),
            });
        }

        // Derive the shared key for this packet
        let user_public_key = MontgomeryPoint(buffer[self.pub_element_range()].try_into()?);
        let shared_key = mix_secret_key * user_public_key;

        // Compute the AEAD and check the Tag, if wrong return Err
        let header_aead_key = ChaCha20Poly1305::new_from_slice(&shared_key.0[..])?;
        let nonce = [0; 12];

        let tag_bytes = buffer[self.tag_range()].to_vec();
        let tag = Tag::from_slice(&tag_bytes);

        header_aead_key
            .decrypt_in_place_detached(
                &nonce.into(),
                &[],
                &mut buffer[self.header_range()],
                tag.as_slice().try_into().unwrap(),
            )
            .map_err(|e| OutfoxError::ChaCha20Poly1305Error(e.to_string()))?;

        // Do a round of LION on the payload
        lion_transform_decrypt(&mut buffer[self.payload_range()], &shared_key.0)?;

        Ok(shared_key)
    }
}
