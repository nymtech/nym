// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::authentication::encrypted_address::EncryptedAddressBytes;
use crate::iv::IV;
use crate::models::CredentialSpendingRequest;
use crate::registration::handshake::SharedKeys;
use crate::{GatewayMacSize, CURRENT_PROTOCOL_VERSION, INITIAL_PROTOCOL_VERSION};
use log::error;
use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use nym_credentials_interface::{CompactEcashError, UnknownCredentialType};
use nym_crypto::generic_array::typenum::Unsigned;
use nym_crypto::hmac::recompute_keyed_hmac_and_verify_tag;
use nym_crypto::symmetric::stream_cipher;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddressError;
use nym_sphinx::forwarding::packet::{MixPacket, MixPacketFormattingError};
use nym_sphinx::params::packet_sizes::PacketSize;
use nym_sphinx::params::{GatewayEncryptionAlgorithm, GatewayIntegrityHmacAlgorithm};
use nym_sphinx::DestinationAddressBytes;
use serde::{Deserialize, Serialize};

use std::str::FromStr;
use std::string::FromUtf8Error;
use thiserror::Error;
use tungstenite::protocol::Message;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RegistrationHandshake {
    HandshakePayload {
        #[serde(default)]
        protocol_version: Option<u8>,
        data: Vec<u8>,
    },
    HandshakeError {
        message: String,
    },
}

impl RegistrationHandshake {
    pub fn new_payload(data: Vec<u8>, will_use_credentials: bool) -> Self {
        // if we're not going to be using credentials, advertise lower protocol version to allow connection
        // to wider range of gateways
        let protocol_version = if will_use_credentials {
            Some(CURRENT_PROTOCOL_VERSION)
        } else {
            Some(INITIAL_PROTOCOL_VERSION)
        };

        RegistrationHandshake::HandshakePayload {
            protocol_version,
            data,
        }
    }

    pub fn new_error<S: Into<String>>(message: S) -> Self {
        RegistrationHandshake::HandshakeError {
            message: message.into(),
        }
    }
}

impl FromStr for RegistrationHandshake {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl TryFrom<String> for RegistrationHandshake {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, serde_json::Error> {
        msg.parse()
    }
}

impl TryInto<String> for RegistrationHandshake {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug, Error)]
pub enum GatewayRequestsError {
    #[error("the request is too short")]
    TooShortRequest,

    #[error("provided MAC is invalid")]
    InvalidMac,

    #[error("address field was incorrectly encoded: {source}")]
    IncorrectlyEncodedAddress {
        #[from]
        source: NymNodeRoutingAddressError,
    },

    #[error("received request had invalid size. (actual: {0}, but expected one of: {} (ACK), {} (REGULAR), {}, {}, {} (EXTENDED))",
        PacketSize::AckPacket.size(),
        PacketSize::RegularPacket.size(),
        PacketSize::ExtendedPacket8.size(),
        PacketSize::ExtendedPacket16.size(),
        PacketSize::ExtendedPacket32.size())
    ]
    RequestOfInvalidSize(usize),

    #[error("received sphinx packet was malformed")]
    MalformedSphinxPacket,

    #[error("the received encrypted data was malformed")]
    MalformedEncryption,

    #[error("provided packet mode is invalid")]
    InvalidPacketMode,

    #[error("provided mix packet was malformed: {source}")]
    InvalidMixPacket {
        #[from]
        source: MixPacketFormattingError,
    },

    #[error("failed to deserialize provided credential: {0}")]
    EcashCredentialDeserializationFailure(#[from] CompactEcashError),

    #[error("failed to deserialize provided credential: EOF")]
    CredentialDeserializationFailureEOF,

    #[error("failed to deserialize provided credential: malformed string: {0}")]
    CredentialDeserializationFailureMalformedString(#[from] FromUtf8Error),

    #[error("failed to deserialize provided credential: {0}")]
    CredentialDeserializationFailureUnknownType(#[from] UnknownCredentialType),

    #[error("the provided [v1] credential has invalid number of parameters - {0}")]
    InvalidNumberOfEmbededParameters(u32),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientControlRequest {
    // TODO: should this also contain a MAC considering that at this point we already
    // have the shared key derived?
    Authenticate {
        #[serde(default)]
        protocol_version: Option<u8>,
        address: String,
        enc_address: String,
        iv: String,
    },
    #[serde(alias = "handshakePayload")]
    RegisterHandshakeInitRequest {
        #[serde(default)]
        protocol_version: Option<u8>,
        data: Vec<u8>,
    },
    BandwidthCredential {
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    },
    BandwidthCredentialV2 {
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    },
    EcashCredential {
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    },
    ClaimFreeTestnetBandwidth,
}

impl ClientControlRequest {
    pub fn new_authenticate(
        address: DestinationAddressBytes,
        enc_address: EncryptedAddressBytes,
        iv: IV,
        uses_credentials: bool,
    ) -> Self {
        // if we're not going to be using credentials, advertise lower protocol version to allow connection
        // to wider range of gateways
        let protocol_version = if uses_credentials {
            Some(CURRENT_PROTOCOL_VERSION)
        } else {
            Some(INITIAL_PROTOCOL_VERSION)
        };

        ClientControlRequest::Authenticate {
            protocol_version,
            address: address.as_base58_string(),
            enc_address: enc_address.to_base58_string(),
            iv: iv.to_base58_string(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ClientControlRequest::Authenticate { .. } => "Authenticate".to_string(),
            ClientControlRequest::RegisterHandshakeInitRequest { .. } => {
                "RegisterHandshakeInitRequest".to_string()
            }
            ClientControlRequest::BandwidthCredential { .. } => "BandwidthCredential".to_string(),
            ClientControlRequest::BandwidthCredentialV2 { .. } => {
                "BandwidthCredentialV2".to_string()
            }
            ClientControlRequest::EcashCredential { .. } => "EcashCredential".to_string(),
            ClientControlRequest::ClaimFreeTestnetBandwidth => {
                "ClaimFreeTestnetBandwidth".to_string()
            }
        }
    }

    pub fn new_enc_ecash_credential(
        credential: CredentialSpendingData,
        shared_key: &SharedKeys,
        iv: IV,
    ) -> Self {
        let cred = CredentialSpendingRequest::new(credential);
        let serialized_credential = cred.to_bytes();
        let enc_credential = shared_key.encrypt_and_tag(&serialized_credential, Some(iv.inner()));

        ClientControlRequest::EcashCredential {
            enc_credential,
            iv: iv.to_bytes(),
        }
    }

    pub fn try_from_enc_ecash_credential(
        enc_credential: Vec<u8>,
        shared_key: &SharedKeys,
        iv: IV,
    ) -> Result<CredentialSpendingRequest, GatewayRequestsError> {
        let credential_bytes = shared_key.decrypt_tagged(&enc_credential, Some(iv.inner()))?;
        CredentialSpendingRequest::try_from_bytes(credential_bytes.as_slice())
            .map_err(|_| GatewayRequestsError::MalformedEncryption)
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
    Authenticate {
        #[serde(default)]
        protocol_version: Option<u8>,
        status: bool,
        bandwidth_remaining: i64,
    },
    Register {
        #[serde(default)]
        protocol_version: Option<u8>,
        status: bool,
    },
    Bandwidth {
        available_total: i64,
    },
    Send {
        remaining_bandwidth: i64,
    },
    Error {
        message: String,
    },
}

impl ServerResponse {
    pub fn name(&self) -> String {
        match self {
            ServerResponse::Authenticate { .. } => "Authenticate".to_string(),
            ServerResponse::Register { .. } => "Register".to_string(),
            ServerResponse::Bandwidth { .. } => "Bandwidth".to_string(),
            ServerResponse::Send { .. } => "Send".to_string(),
            ServerResponse::Error { .. } => "Error".to_string(),
        }
    }
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
                let forwarding_data = match mix_packet.into_bytes() {
                    Ok(mix_packet) => mix_packet,
                    Err(e) => {
                        error!("Could not convert packet to bytes: {e}");
                        return vec![];
                    }
                };

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
            shared_keys.mac_key().as_slice(),
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
        let handshake_payload_with_protocol = RegistrationHandshake::HandshakePayload {
            protocol_version: Some(42),
            data: handshake_data.clone(),
        };
        let serialized = serde_json::to_string(&handshake_payload_with_protocol).unwrap();
        let deserialized = ClientControlRequest::try_from(serialized).unwrap();

        match deserialized {
            ClientControlRequest::RegisterHandshakeInitRequest {
                protocol_version,
                data,
            } => {
                assert_eq!(protocol_version, Some(42));
                assert_eq!(data, handshake_data)
            }
            _ => unreachable!("this branch shouldn't have been reached!"),
        }

        let handshake_payload_without_protocol = RegistrationHandshake::HandshakePayload {
            protocol_version: None,
            data: handshake_data.clone(),
        };
        let serialized = serde_json::to_string(&handshake_payload_without_protocol).unwrap();
        let deserialized = ClientControlRequest::try_from(serialized).unwrap();

        match deserialized {
            ClientControlRequest::RegisterHandshakeInitRequest {
                protocol_version,
                data,
            } => {
                assert!(protocol_version.is_none());
                assert_eq!(data, handshake_data)
            }
            _ => unreachable!("this branch shouldn't have been reached!"),
        }
    }
}
