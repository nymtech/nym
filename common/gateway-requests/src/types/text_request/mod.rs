// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CredentialSpendingRequest;
use crate::text_request::authenticate::AuthenticateRequest;
use crate::{
    GatewayRequestsError, SharedGatewayKey, SymmetricKey, AES_GCM_SIV_PROTOCOL_VERSION,
    AUTHENTICATE_V2_PROTOCOL_VERSION, CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION,
    INITIAL_PROTOCOL_VERSION,
};
#[cfg(feature = "otel")]
use nym_bin_common::opentelemetry::context::ContextCarrier;
use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::ed25519;
use nym_sphinx::DestinationAddressBytes;
use nym_statistics_common::types::SessionType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{instrument, warn};
use tungstenite::Message;

pub mod authenticate;

// wrapper for all encrypted requests for ease of use
#[derive(Serialize, Deserialize, Debug, Clone)]
#[non_exhaustive]
pub enum ClientRequest {
    UpgradeKey {
        hkdf_salt: Vec<u8>,
        derived_key_digest: Vec<u8>,
    },
    ForgetMe {
        client: bool,
        stats: bool,
    },
    RememberMe {
        session_type: SessionType,
    },
}

impl ClientRequest {
    pub fn encrypt<S: SymmetricKey>(
        &self,
        key: &S,
    ) -> Result<ClientControlRequest, GatewayRequestsError> {
        // we're using json representation for few reasons:
        // - ease of re-implementation in other languages (compared to for example bincode)
        // - we expect all requests to be relatively small - for anything bigger use BinaryRequest!
        // - the schema is self-describing which simplifies deserialisation

        // SAFETY: the trait has been derived correctly with no weird variants
        let plaintext = serde_json::to_vec(self).unwrap();
        let nonce = key.random_nonce_or_iv();
        let ciphertext = key.encrypt(&plaintext, Some(&nonce))?;
        Ok(ClientControlRequest::EncryptedRequest { ciphertext, nonce })
    }

    pub fn decrypt<S: SymmetricKey>(
        ciphertext: &[u8],
        nonce: &[u8],
        key: &S,
    ) -> Result<Self, GatewayRequestsError> {
        let plaintext = key.decrypt(ciphertext, Some(nonce))?;
        serde_json::from_slice(&plaintext)
            .map_err(|source| GatewayRequestsError::MalformedRequest { source })
    }
}

// if you're adding new variants here, consider putting them inside `ClientRequest` instead
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ClientControlRequest {
    // TODO: should this also contain a MAC considering that at this point we already
    // have the shared key derived?
    Authenticate {
        #[serde(default)]
        protocol_version: Option<u8>,
        address: String,
        enc_address: String,
        iv: String,
        /// this is a trace id that is used in testing and performance verification
        /// in mainnet, this will always be set to None
        #[serde(default)]
        otel_context: Option<HashMap<String, String>>,
    },


    AuthenticateV2(Box<AuthenticateRequest>),

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
    EncryptedRequest {
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
    },
    SupportedProtocol {},
    // if you're adding new variants here, consider putting them inside `ClientRequest` instead
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

        #[cfg(feature = "otel")]
        let context_carrier = {
            let context = opentelemetry::Context::current();
            ContextCarrier::new_with_current_context(context).into_map()
        };

        Ok(ClientControlRequest::Authenticate {
            protocol_version,
            address: address.as_base58_string(),
            enc_address: bs58::encode(&ciphertext).into_string(),
            iv: bs58::encode(&nonce).into_string(),
            #[cfg(feature = "otel")]
            otel_context: Some(context_carrier),
            #[cfg(not(feature = "otel"))]
            otel_context: None,
        })
    }

    #[instrument]
    pub fn new_authenticate_v2(
        shared_key: &SharedGatewayKey,
        identity_keys: &ed25519::KeyPair,
    ) -> Result<Self, GatewayRequestsError> {
        // if we're using v2 authentication, we must announce at least that protocol version
        let protocol_version = AUTHENTICATE_V2_PROTOCOL_VERSION;

        #[cfg(feature = "otel")]
        let context_carrier = {
            use nym_bin_common::opentelemetry::context::extract_trace_id_from_tracing_cx;
            let trace_id = extract_trace_id_from_tracing_cx();

            use tracing_opentelemetry::OpenTelemetrySpanExt;

            let current_span = tracing::Span::current();
            let otel_context = current_span.context();
            ContextCarrier::new_with_current_context(otel_context).into_map()
        };
        #[cfg(not(feature = "otel"))]
        let context_carrier: HashMap<String, String> = HashMap::new();

        Ok(ClientControlRequest::AuthenticateV2(Box::new(
            AuthenticateRequest::new(
                protocol_version,
                shared_key,
                identity_keys,
                Some(context_carrier)
            )?,
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
