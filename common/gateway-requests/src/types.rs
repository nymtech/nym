// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CredentialSpendingRequest;
use crate::registration::handshake::{SharedGatewayKey, SharedKeyUsageError};
use crate::{
    AES_GCM_SIV_PROTOCOL_VERSION, CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION, INITIAL_PROTOCOL_VERSION,
};
use nym_credentials::ecash::bandwidth::CredentialSpendingData;
use nym_credentials_interface::CompactEcashError;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddressError;
use nym_sphinx::forwarding::packet::{MixPacket, MixPacketFormattingError};
use nym_sphinx::params::packet_sizes::PacketSize;
use nym_sphinx::DestinationAddressBytes;
use serde::{Deserialize, Serialize};
use std::iter::once;
use std::str::FromStr;
use std::string::FromUtf8Error;
use strum::FromRepr;
use thiserror::Error;
use tracing::log::error;
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
    pub fn new_payload(data: Vec<u8>, protocol_version: u8) -> Self {
        RegistrationHandshake::HandshakePayload {
            protocol_version: Some(protocol_version),
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

// specific errors (that should not be nested!!) for clients to match on
#[derive(Debug, Copy, Clone, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimpleGatewayRequestsError {
    #[error("insufficient bandwidth available to process the request. required: {required}B, available: {available}B")]
    OutOfBandwidth { required: i64, available: i64 },

    #[error("the provided ticket has already been spent before at this gateway")]
    TicketReplay,
}

impl SimpleGatewayRequestsError {
    pub fn is_ticket_replay(&self) -> bool {
        matches!(self, SimpleGatewayRequestsError::TicketReplay)
    }
}

#[derive(Debug, Error)]
pub enum GatewayRequestsError {
    #[error(transparent)]
    KeyUsageFailure(#[from] SharedKeyUsageError),

    #[error("received request with an unknown kind: {kind}")]
    UnknownRequestKind { kind: u8 },

    #[error("received response with an unknown kind: {kind}")]
    UnknownResponseKind { kind: u8 },

    #[error("the encryption flag had an unexpected value")]
    InvalidEncryptionFlag,

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

    #[error("failed to serialise created sphinx packet: {0}")]
    SphinxSerialisationFailure(#[from] MixPacketFormattingError),

    #[error("the received encrypted data was malformed")]
    MalformedEncryption,

    #[error("provided packet mode is invalid")]
    InvalidPacketMode,

    #[error("failed to deserialize provided credential: {0}")]
    EcashCredentialDeserializationFailure(#[from] CompactEcashError),

    #[error("failed to deserialize provided credential: EOF")]
    CredentialDeserializationFailureEOF,

    #[error("failed to deserialize provided credential: malformed string: {0}")]
    CredentialDeserializationFailureMalformedString(#[from] FromUtf8Error),

    #[error("the provided [v1] credential has invalid number of parameters - {0}")]
    InvalidNumberOfEmbededParameters(u32),

    // variant to catch legacy errors
    #[error("{0}")]
    Other(String),
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
    SupportedProtocol {},
}

impl ClientControlRequest {
    pub fn new_authenticate(
        address: DestinationAddressBytes,
        shared_key: &SharedGatewayKey,
        uses_credentials: bool,
    ) -> Result<Self, GatewayRequestsError> {
        // if we're encrypting with non-legacy key, the remote must support AES256-GCM-SIV
        let protocol_version = if !shared_key.is_legacy() {
            Some(AES_GCM_SIV_PROTOCOL_VERSION)
        } else if uses_credentials {
            Some(CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION)
        } else {
            // if we're not going to be using credentials, advertise lower protocol version to allow connection
            // to wider range of gateways
            Some(INITIAL_PROTOCOL_VERSION)
        };

        let nonce = shared_key.random_nonce_or_iv();
        let ciphertext = shared_key.encrypt_naive(address.as_bytes_ref(), Some(&nonce))?;

        Ok(ClientControlRequest::Authenticate {
            protocol_version,
            address: address.as_base58_string(),
            enc_address: bs58::encode(&ciphertext).into_string(),
            iv: bs58::encode(&nonce).into_string(),
        })
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
            ClientControlRequest::SupportedProtocol { .. } => "SupportedProtocol".to_string(),
        }
    }

    pub fn new_enc_ecash_credential(
        credential: CredentialSpendingData,
        shared_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        let cred = CredentialSpendingRequest::new(credential);
        let serialized_credential = cred.to_bytes();

        let nonce = shared_key.random_nonce_or_iv();
        let enc_credential = shared_key.encrypt(&serialized_credential, Some(&nonce))?;

        Ok(ClientControlRequest::EcashCredential {
            enc_credential,
            iv: nonce,
        })
    }

    pub fn try_from_enc_ecash_credential(
        enc_credential: Vec<u8>,
        shared_key: &SharedGatewayKey,
        iv: Vec<u8>,
    ) -> Result<CredentialSpendingRequest, GatewayRequestsError> {
        let credential_bytes = shared_key.decrypt(&enc_credential, Some(&iv))?;
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
        msg.parse()
    }
}

impl FromStr for ClientControlRequest {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
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
    SupportedProtocol {
        version: u8,
    },
    // Generic error
    Error {
        message: String,
    },
    // Specific typed errors
    // so that clients could match on this variant without doing naive string matching
    TypedError {
        error: SimpleGatewayRequestsError,
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
            ServerResponse::TypedError { .. } => "TypedError".to_string(),
            ServerResponse::SupportedProtocol { .. } => "SupportedProtocol".to_string(),
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

// each binary message consists of the following structure (for non-legacy messages)
// KIND || ENC_FLAG || MAYBE_NONCE || CIPHERTEXT/PLAINTEXT
// first byte is the kind of data to influence further serialisation/deseralisation
// second byte is a flag indicating whether the content is encrypted
// then it's followed by a pseudorandom nonce, assuming encryption is used
// finally, the rest of the message is the associated ciphertext or just plaintext (if message wasn't encrypted)
pub struct BinaryData<'a> {
    kind: u8,
    encrypted: bool,
    maybe_nonce: Option<&'a [u8]>,
    data: &'a [u8],
}

impl<'a> BinaryData<'a> {
    // serialises possibly encrypted data into bytes to be put on the wire
    pub fn into_raw(self, legacy: bool) -> Vec<u8> {
        if legacy {
            return self.data.to_vec();
        }

        let i = once(self.kind).chain(once(if self.encrypted { 1 } else { 0 }));
        if let Some(nonce) = self.maybe_nonce {
            i.chain(nonce.iter().copied())
                .chain(self.data.iter().copied())
                .collect()
        } else {
            i.chain(self.data.iter().copied()).collect()
        }
    }

    // attempts to perform basic parsing on bytes received from the wire
    pub fn from_raw(
        raw: &'a [u8],
        available_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        // if we're using legacy key, it's quite simple:
        // it's always encrypted with no nonce and the request/response kind is always 1
        if available_key.is_legacy() {
            return Ok(BinaryData {
                kind: 1,
                encrypted: true,
                maybe_nonce: None,
                data: raw,
            });
        }

        if raw.len() < 2 {
            return Err(GatewayRequestsError::TooShortRequest);
        }

        let kind = raw[0];
        let encrypted = if raw[1] == 1 {
            true
        } else if raw[1] == 0 {
            false
        } else {
            return Err(GatewayRequestsError::InvalidEncryptionFlag);
        };

        // if data is encrypted, there MUST be a nonce present for non-legacy keys
        if encrypted && raw.len() < available_key.nonce_size() + 2 {
            return Err(GatewayRequestsError::TooShortRequest);
        }

        Ok(BinaryData {
            kind,
            encrypted,
            maybe_nonce: Some(&raw[2..2 + available_key.nonce_size()]),
            data: &raw[2 + available_key.nonce_size()..],
        })
    }

    // attempt to encrypt plaintext of provided response/request and serialise it into wire format
    pub fn make_encrypted_blob(
        kind: u8,
        plaintext: &[u8],
        key: &SharedGatewayKey,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        let maybe_nonce = key.random_nonce_or_zero_iv();

        let ciphertext = key.encrypt(plaintext, maybe_nonce.as_deref())?;
        Ok(BinaryData {
            kind,
            encrypted: true,
            maybe_nonce: maybe_nonce.as_deref(),
            data: &ciphertext,
        }
        .into_raw(key.is_legacy()))
    }

    // attempts to parse previously recovered bytes into a [`BinaryRequest`]
    pub fn into_request(
        self,
        key: &SharedGatewayKey,
    ) -> Result<BinaryRequest, GatewayRequestsError> {
        let kind = BinaryRequestKind::from_repr(self.kind)
            .ok_or(GatewayRequestsError::UnknownRequestKind { kind: self.kind })?;

        let plaintext = if self.encrypted {
            &*key.decrypt(self.data, self.maybe_nonce)?
        } else {
            self.data
        };

        BinaryRequest::from_plaintext(kind, plaintext)
    }

    // attempts to parse previously recovered bytes into a [`BinaryResponse`]
    pub fn into_response(
        self,
        key: &SharedGatewayKey,
    ) -> Result<BinaryResponse, GatewayRequestsError> {
        let kind = BinaryResponseKind::from_repr(self.kind)
            .ok_or(GatewayRequestsError::UnknownResponseKind { kind: self.kind })?;

        let plaintext = if self.encrypted {
            &*key.decrypt(self.data, self.maybe_nonce)?
        } else {
            self.data
        };

        BinaryResponse::from_plaintext(kind, plaintext)
    }
}

// in legacy mode requests use zero IV without
pub enum BinaryRequest {
    ForwardSphinx { packet: MixPacket },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, FromRepr, PartialEq)]
#[non_exhaustive]
pub enum BinaryRequestKind {
    ForwardSphinx = 1,
}

// Right now the only valid `BinaryRequest` is a request to forward a sphinx packet.
// It is encrypted using the derived shared key between client and the gateway. Thanks to
// randomness inside the sphinx packet themselves (even via the same route), the 0s IV can be used here.
// HOWEVER, NOTE: If we introduced another 'BinaryRequest', we must carefully examine if a 0s IV
// would work there.
impl BinaryRequest {
    pub fn kind(&self) -> BinaryRequestKind {
        match self {
            BinaryRequest::ForwardSphinx { .. } => BinaryRequestKind::ForwardSphinx,
        }
    }

    pub fn from_plaintext(
        kind: BinaryRequestKind,
        plaintext: &[u8],
    ) -> Result<Self, GatewayRequestsError> {
        match kind {
            BinaryRequestKind::ForwardSphinx => {
                let packet = MixPacket::try_from_bytes(plaintext)?;
                Ok(BinaryRequest::ForwardSphinx { packet })
            }
        }
    }

    pub fn try_from_encrypted_tagged_bytes(
        bytes: Vec<u8>,
        shared_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        BinaryData::from_raw(&bytes, shared_key)?.into_request(shared_key)
    }

    pub fn into_encrypted_tagged_bytes(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        let kind = self.kind();

        let plaintext = match self {
            BinaryRequest::ForwardSphinx { packet } => packet.into_bytes()?,
        };

        BinaryData::make_encrypted_blob(kind as u8, &plaintext, shared_key)
    }

    pub fn into_ws_message(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Message, GatewayRequestsError> {
        // all variants are currently encrypted
        let blob = match self {
            BinaryRequest::ForwardSphinx { .. } => self.into_encrypted_tagged_bytes(shared_key)?,
        };

        Ok(Message::Binary(blob))
    }
}

pub enum BinaryResponse {
    PushedMixMessage { message: Vec<u8> },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, FromRepr, PartialEq)]
#[non_exhaustive]
pub enum BinaryResponseKind {
    PushedMixMessage = 1,
}

impl BinaryResponse {
    pub fn kind(&self) -> BinaryResponseKind {
        match self {
            BinaryResponse::PushedMixMessage { .. } => BinaryResponseKind::PushedMixMessage,
        }
    }

    pub fn from_plaintext(
        kind: BinaryResponseKind,
        plaintext: &[u8],
    ) -> Result<Self, GatewayRequestsError> {
        match kind {
            BinaryResponseKind::PushedMixMessage => Ok(BinaryResponse::PushedMixMessage {
                message: plaintext.to_vec(),
            }),
        }
    }

    pub fn try_from_encrypted_tagged_bytes(
        bytes: Vec<u8>,
        shared_key: &SharedGatewayKey,
    ) -> Result<Self, GatewayRequestsError> {
        BinaryData::from_raw(&bytes, shared_key)?.into_response(shared_key)
    }

    pub fn into_encrypted_tagged_bytes(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Vec<u8>, GatewayRequestsError> {
        let kind = self.kind();

        let plaintext = match self {
            BinaryResponse::PushedMixMessage { message } => message,
        };

        BinaryData::make_encrypted_blob(kind as u8, &plaintext, shared_key)
    }

    pub fn into_ws_message(
        self,
        shared_key: &SharedGatewayKey,
    ) -> Result<Message, GatewayRequestsError> {
        // all variants are currently encrypted
        let blob = match self {
            BinaryResponse::PushedMixMessage { .. } => {
                self.into_encrypted_tagged_bytes(shared_key)?
            }
        };

        Ok(Message::Binary(blob))
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
