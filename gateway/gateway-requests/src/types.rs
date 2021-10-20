// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::authentication::encrypted_address::EncryptedAddressBytes;
use crate::iv::IV;
use crate::registration::handshake::SharedKeys;
use crate::GatewayMacSize;
use crypto::generic_array::typenum::Unsigned;
use crypto::hmac::recompute_keyed_hmac_and_verify_tag;
use crypto::symmetric::stream_cipher;
use nymsphinx::addressing::nodes::NymNodeRoutingAddressError;
use nymsphinx::forwarding::packet::{MixPacket, MixPacketFormattingError};
use nymsphinx::params::packet_sizes::PacketSize;
use nymsphinx::params::{GatewayEncryptionAlgorithm, GatewayIntegrityHmacAlgorithm};
use nymsphinx::DestinationAddressBytes;
use serde::{Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{self, Error, Formatter},
};
use tungstenite::protocol::Message;

#[cfg(feature = "coconut")]
use coconut_interface::Credential;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RegistrationHandshake {
    HandshakePayload { data: Vec<u8> },
    HandshakeError { message: String },
}

impl RegistrationHandshake {
    pub fn new_payload(data: Vec<u8>) -> Self {
        RegistrationHandshake::HandshakePayload { data }
    }

    pub fn new_error<S: Into<String>>(message: S) -> Self {
        RegistrationHandshake::HandshakeError {
            message: message.into(),
        }
    }
}

impl TryFrom<String> for RegistrationHandshake {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, serde_json::Error> {
        serde_json::from_str(&msg)
    }
}

impl TryInto<String> for RegistrationHandshake {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug)]
pub enum GatewayRequestsError {
    TooShortRequest,
    InvalidMac,
    IncorrectlyEncodedAddress,
    RequestOfInvalidSize(usize),
    MalformedSphinxPacket,
    MalformedEncryption,
    InvalidPacketMode,
    InvalidMixPacket(MixPacketFormattingError),
}

impl fmt::Display for GatewayRequestsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use GatewayRequestsError::*;
        match self {
            TooShortRequest => write!(f, "the request is too short"),
            InvalidMac => write!(f, "provided MAC is invalid"),
            IncorrectlyEncodedAddress => write!(f, "address field was incorrectly encoded"),
            RequestOfInvalidSize(actual) =>
                write!(
                f,
                "received request had invalid size. (actual: {}, but expected one of: {} (ACK), {} (REGULAR), {} (EXTENDED))",
                actual, PacketSize::AckPacket.size(), PacketSize::RegularPacket.size(), PacketSize::ExtendedPacket.size()
            ),
            MalformedSphinxPacket => write!(f, "received sphinx packet was malformed"),
            MalformedEncryption => write!(f, "the received encrypted data was malformed"),
            InvalidPacketMode => write!(f, "provided packet mode is invalid"),
            InvalidMixPacket(err) => write!(f, "provided mix packet was malformed - {}", err)
        }
    }
}

impl std::error::Error for GatewayRequestsError {}

impl From<NymNodeRoutingAddressError> for GatewayRequestsError {
    fn from(_: NymNodeRoutingAddressError) -> Self {
        GatewayRequestsError::IncorrectlyEncodedAddress
    }
}

impl From<MixPacketFormattingError> for GatewayRequestsError {
    fn from(err: MixPacketFormattingError) -> Self {
        GatewayRequestsError::InvalidMixPacket(err)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientControlRequest {
    // TODO: should this also contain a MAC considering that at this point we already
    // have the shared key derived?
    Authenticate {
        address: String,
        enc_address: String,
        iv: String,
    },
    #[serde(alias = "handshakePayload")]
    RegisterHandshakeInitRequest { data: Vec<u8> },
    #[cfg(feature = "coconut")]
    CoconutBandwidthCredential {
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    },
}

impl ClientControlRequest {
    pub fn new_authenticate(
        address: DestinationAddressBytes,
        enc_address: EncryptedAddressBytes,
        iv: IV,
    ) -> Self {
        ClientControlRequest::Authenticate {
            address: address.as_base58_string(),
            enc_address: enc_address.to_base58_string(),
            iv: iv.to_base58_string(),
        }
    }

    #[cfg(feature = "coconut")]
    pub fn new_enc_coconut_bandwidth_credential(
        credential: &Credential,
        shared_key: &SharedKeys,
        iv: IV,
    ) -> Option<Self> {
        match bincode::serialize(credential) {
            Ok(serialized_credential) => {
                let enc_credential =
                    shared_key.encrypt_and_tag(&serialized_credential, Some(iv.inner()));

                Some(ClientControlRequest::CoconutBandwidthCredential {
                    enc_credential,
                    iv: iv.to_bytes(),
                })
            }
            _ => None,
        }
    }

    #[cfg(feature = "coconut")]
    pub fn try_from_enc_coconut_bandwidth_credential(
        enc_credential: Vec<u8>,
        shared_key: &SharedKeys,
        iv: IV,
    ) -> Result<Credential, GatewayRequestsError> {
        let credential = shared_key.decrypt_tagged(&enc_credential, Some(iv.inner()))?;
        bincode::deserialize(&credential).map_err(|_| GatewayRequestsError::MalformedEncryption)
    }
}

impl From<ClientControlRequest> for Message {
    fn from(req: ClientControlRequest) -> Self {
        // it should be safe to call `unwrap` here as the message is generated by the server
        // so if it fails (and consequently panics) it's a bug that should be resolved
        let str_req = serde_json::to_string(&req).unwrap();
        Message::Text(str_req)
    }
}

impl TryFrom<String> for ClientControlRequest {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl TryInto<String> for ClientControlRequest {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerResponse {
    Authenticate { status: bool },
    Register { status: bool },
    Bandwidth { available_total: i64 },
    Send { remaining_bandwidth: i64 },
    Error { message: String },
}

impl ServerResponse {
    pub fn new_error<S: Into<String>>(msg: S) -> Self {
        ServerResponse::Error {
            message: msg.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ServerResponse::Error { .. })
    }

    pub fn implies_successful_authentication(&self) -> bool {
        match self {
            ServerResponse::Authenticate { status, .. } => *status,
            ServerResponse::Register { status, .. } => *status,
            _ => false,
        }
    }
}

impl From<ServerResponse> for Message {
    fn from(res: ServerResponse) -> Self {
        // it should be safe to call `unwrap` here as the message is generated by the server
        // so if it fails (and consequently panics) it's a bug that should be resolved
        let str_res = serde_json::to_string(&res).unwrap();
        Message::Text(str_res)
    }
}

impl TryFrom<String> for ServerResponse {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, serde_json::Error> {
        serde_json::from_str(&msg)
    }
}

pub enum BinaryRequest {
    ForwardSphinx(MixPacket),
}

// Right now the only valid `BinaryRequest` is a request to forward a sphinx packet.
// It is encrypted using the derived shared key between client and the gateway. Thanks to
// randomness inside the sphinx packet themselves (even via the same route), the 0s IV can be used here.
// HOWEVER, NOTE: If we introduced another 'BinaryRequest', we must carefully examine if a 0s IV
// would work there.
impl BinaryRequest {
    pub fn try_from_encrypted_tagged_bytes(
        raw_req: Vec<u8>,
        shared_keys: &SharedKeys,
    ) -> Result<Self, GatewayRequestsError> {
        let message_bytes = &shared_keys.decrypt_tagged(&raw_req, None)?;

        // right now there's only a single option possible which significantly simplifies the logic
        // if we decided to allow for more 'binary' messages, the API wouldn't need to change.
        let mix_packet = MixPacket::try_from_bytes(message_bytes)?;
        Ok(BinaryRequest::ForwardSphinx(mix_packet))
    }

    pub fn into_encrypted_tagged_bytes(self, shared_key: &SharedKeys) -> Vec<u8> {
        match self {
            BinaryRequest::ForwardSphinx(mix_packet) => {
                let forwarding_data = mix_packet.into_bytes();

                // TODO: it could be theoretically slightly more efficient if the data wasn't taken
                // by reference because then it makes a copy for encryption rather than do it in place
                shared_key.encrypt_and_tag(&forwarding_data, None)
            }
        }
    }

    // TODO: this will be encrypted, etc.
    pub fn new_forward_request(mix_packet: MixPacket) -> BinaryRequest {
        BinaryRequest::ForwardSphinx(mix_packet)
    }

    pub fn into_ws_message(self, shared_key: &SharedKeys) -> Message {
        Message::Binary(self.into_encrypted_tagged_bytes(shared_key))
    }
}

// Introduced for consistency sake
pub enum BinaryResponse {
    PushedMixMessage(Vec<u8>),
}

impl BinaryResponse {
    pub fn try_from_encrypted_tagged_bytes(
        raw_req: Vec<u8>,
        shared_keys: &SharedKeys,
    ) -> Result<Self, GatewayRequestsError> {
        let mac_size = GatewayMacSize::to_usize();
        if raw_req.len() < mac_size {
            return Err(GatewayRequestsError::TooShortRequest);
        }

        let mac_tag = &raw_req[..mac_size];
        let message_bytes = &raw_req[mac_size..];

        if !recompute_keyed_hmac_and_verify_tag::<GatewayIntegrityHmacAlgorithm>(
            shared_keys.mac_key(),
            message_bytes,
            mac_tag,
        ) {
            return Err(GatewayRequestsError::InvalidMac);
        }

        let zero_iv = stream_cipher::zero_iv::<GatewayEncryptionAlgorithm>();
        let plaintext = stream_cipher::decrypt::<GatewayEncryptionAlgorithm>(
            shared_keys.encryption_key(),
            &zero_iv,
            message_bytes,
        );

        Ok(BinaryResponse::PushedMixMessage(plaintext))
    }

    pub fn into_encrypted_tagged_bytes(self, shared_key: &SharedKeys) -> Vec<u8> {
        match self {
            // TODO: it could be theoretically slightly more efficient if the data wasn't taken
            // by reference because then it makes a copy for encryption rather than do it in place
            BinaryResponse::PushedMixMessage(message) => shared_key.encrypt_and_tag(&message, None),
        }
    }

    pub fn new_pushed_mix_message(msg: Vec<u8>) -> Self {
        BinaryResponse::PushedMixMessage(msg)
    }

    pub fn into_ws_message(self, shared_key: &SharedKeys) -> Message {
        Message::Binary(self.into_encrypted_tagged_bytes(shared_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_payload_can_be_deserialized_into_register_handshake_init_request() {
        let handshake_data = vec![1, 2, 3, 4, 5, 6];
        let handshake_payload = RegistrationHandshake::HandshakePayload {
            data: handshake_data.clone(),
        };
        let serialized = serde_json::to_string(&handshake_payload).unwrap();
        let deserialized = ClientControlRequest::try_from(serialized).unwrap();

        match deserialized {
            ClientControlRequest::RegisterHandshakeInitRequest { data } => {
                assert_eq!(data, handshake_data)
            }
            _ => unreachable!("this branch shouldn't have been reached!"),
        }
    }
}
