// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CredentialSpendingRequest;
use crate::text_request::authenticate::AuthenticateRequest;
use crate::{GatewayRequestsError, SharedSymmetricKey, AUTHENTICATE_V2_PROTOCOL_VERSION};
use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tungstenite::Message;

pub mod authenticate;

// wrapper for all encrypted requests for ease of use
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum ClientRequest {
    ForgetMe { client: bool, stats: bool },
}

impl ClientRequest {
    pub fn encrypt(
        &self,
        key: &SharedSymmetricKey,
    ) -> Result<ClientControlRequest, GatewayRequestsError> {
        // we're using json representation for few reasons:
        // - ease of re-implementation in other languages (compared to for example bincode)
        // - we expect all requests to be relatively small - for anything bigger use BinaryRequest!
        // - the schema is self-describing which simplifies deserialisation

        // SAFETY: the trait has been derived correctly with no weird variants
        let plaintext = serde_json::to_vec(self).unwrap();
        let nonce = key.random_nonce();
        let ciphertext = key.encrypt(&plaintext, &nonce)?;
        Ok(ClientControlRequest::EncryptedRequest {
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    pub fn decrypt(
        ciphertext: &[u8],
        nonce: &[u8],
        key: &SharedSymmetricKey,
    ) -> Result<Self, GatewayRequestsError> {
        let nonce = SharedSymmetricKey::validate_aead_nonce(nonce)?;
        let plaintext = key.decrypt(ciphertext, &nonce)?;
        serde_json::from_slice(&plaintext)
            .map_err(|source| GatewayRequestsError::MalformedRequest { source })
    }
}

// if you're adding new variants here, consider putting them inside `ClientRequest` instead
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ClientControlRequest {
    AuthenticateV2(Box<AuthenticateRequest>),

    #[serde(alias = "handshakePayload")]
    RegisterHandshakeInitRequest {
        protocol_version: u8,
        data: Vec<u8>,
    },

    EcashCredential {
        enc_credential: Vec<u8>,
        #[serde(alias = "iv")]
        nonce: Vec<u8>,
    },
    ClaimFreeTestnetBandwidth,
    EncryptedRequest {
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
    },
    SupportedProtocol {},
    // if you're adding new variants here, consider putting them inside `ClientRequest` instead

    // NO LONGER SUPPORTED:
    Authenticate {
        #[serde(default)]
        protocol_version: Option<u8>,
        address: String,
        enc_address: String,
        iv: String,
    },

    BandwidthCredential {
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    },
    BandwidthCredentialV2 {
        enc_credential: Vec<u8>,
        iv: Vec<u8>,
    },
}

impl ClientControlRequest {
    pub fn new_authenticate_v2(
        shared_key: &SharedSymmetricKey,
        identity_keys: &ed25519::KeyPair,
    ) -> Result<Self, GatewayRequestsError> {
        // if we're using v2 authentication, we must announce at least that protocol version
        // (which also implicitly implies usage of AES256-GCM-SIV
        let protocol_version = AUTHENTICATE_V2_PROTOCOL_VERSION;

        Ok(ClientControlRequest::AuthenticateV2(Box::new(
            AuthenticateRequest::new(protocol_version, shared_key, identity_keys)?,
        )))
    }

    pub fn name(&self) -> String {
        match self {
            ClientControlRequest::Authenticate { .. } => "Authenticate".to_string(),
            ClientControlRequest::AuthenticateV2(..) => "AuthenticateV2".to_string(),
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
            ClientControlRequest::EncryptedRequest { .. } => "EncryptedRequest".to_string(),
        }
    }

    pub fn new_enc_ecash_credential(
        credential: CredentialSpendingData,
        shared_key: &SharedSymmetricKey,
    ) -> Result<Self, GatewayRequestsError> {
        let cred = CredentialSpendingRequest::new(credential);
        let serialized_credential = cred.to_bytes();

        let nonce = shared_key.random_nonce();
        let enc_credential = shared_key.encrypt(&serialized_credential, &nonce)?;

        Ok(ClientControlRequest::EcashCredential {
            enc_credential,
            nonce: nonce.to_vec(),
        })
    }

    pub fn try_from_enc_ecash_credential(
        enc_credential: Vec<u8>,
        shared_key: &SharedSymmetricKey,
        nonce: Vec<u8>,
    ) -> Result<CredentialSpendingRequest, GatewayRequestsError> {
        let nonce = SharedSymmetricKey::validate_aead_nonce(&nonce)?;
        let credential_bytes = shared_key.decrypt(&enc_credential, &nonce)?;
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
