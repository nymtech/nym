//! This module contains all the types that are used in the API.
//!
//! - `ApiEphemeraMessage`
//! - `RawApiEphemeraMessage`
//! - `ApiBlock`
//! - `ApiCertificate`
//! - `Health`
//! - `ApiError`
//! - `ApiEphemeraConfig`
//! - `ApiDhtQueryRequest`
//! - `ApiDhtQueryResponse`
//! - `ApiDhtStoreRequest`
//! - `ApiBroadcastInfo`
//! - `ApiBlockBroadcastInfo`
//! - `ApiVerifyMessageInBlock`

use std::collections::HashSet;
use std::fmt::Display;

use array_bytes::{bytes2hex, hex2bytes};
use log::error;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

use crate::peer::{PeerId, ToPeerId};
use crate::utilities::codec::{Codec, DecodingError, EncodingError, EphemeraCodec};
use crate::{
    block::types::{block::Block, block::BlockHeader, message::EphemeraMessage},
    codec::{Decode, Encode},
    crypto::{Keypair, PublicKey},
    ephemera_api,
    utilities::{
        crypto::{Certificate, Signature},
        time::EphemeraTime,
    },
};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Application rejected ephemera message")]
    ApplicationRejectedMessage,
    #[error("Duplicate message")]
    DuplicateMessage,
    #[error("Invalid hash: {0}")]
    InvalidHash(String),
    #[error("ApplicationError: {0}")]
    Application(#[from] ephemera_api::ApplicationError),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// # Ephemera message.
///
/// A message submitted to an Ephemera node will be gossiped to other nodes.
/// And will be eventually included in a Ephemera block.
///
/// It needs to signed by the sender. The signature is included in the certificate.
///
/// The fields of the message what are signed:
/// - timestamp
/// - label
/// - data
///
/// Currently it's up provided [`ephemera_api::application::Application`] to verify the signature.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiEphemeraMessage {
    /// The timestamp of the message.
    pub timestamp: u64,
    /// The label of the message. It can be used to identify the type of a message for example.
    pub label: String,
    /// The data of the message. It is application specific.
    pub data: Vec<u8>,
    /// The certificate of the message. All messages are required to be signed.
    pub certificate: ApiCertificate,
}

impl ApiEphemeraMessage {
    #[must_use]
    pub fn new(raw_message: RawApiEphemeraMessage, certificate: ApiCertificate) -> Self {
        Self {
            timestamp: raw_message.timestamp,
            label: raw_message.label,
            data: raw_message.data,
            certificate,
        }
    }

    /// Generates the message hash.
    ///
    /// # Errors
    /// - If internal hash function fails.
    pub fn hash(&self) -> anyhow::Result<String> {
        let em = EphemeraMessage::from(self.clone());
        let hash = em.hash_with_default_hasher()?.to_string();
        Ok(hash)
    }
}

/// `RawApiEphemeraMessage` contains the fields of the `ApiEphemeraMessage` that are signed.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct RawApiEphemeraMessage {
    /// The timestamp of the message. It's initialized when the message is created.
    /// It uses UTC time.
    pub timestamp: u64,
    /// The label of the message. It can be used to identify the type of a message without decoding full data.
    pub label: String,
    /// The data of the message. It is application specific.
    pub data: Vec<u8>,
}

impl RawApiEphemeraMessage {
    #[must_use]
    pub fn new(label: String, data: Vec<u8>) -> Self {
        Self {
            timestamp: EphemeraTime::now(),
            label,
            data,
        }
    }

    /// Signs the message with the given keypair.
    ///
    /// # Signing example
    ///
    /// ```
    /// use ephemera::codec::Encode;
    /// use ephemera::crypto::{EphemeraKeypair, EphemeraPublicKey, Keypair};
    /// use ephemera::ephemera_api::{ApiEphemeraMessage, RawApiEphemeraMessage};
    ///
    /// let keypair = Keypair::generate(None);
    /// let raw_message = RawApiEphemeraMessage::new("test".to_string(), vec![]);
    ///
    /// let signed_message:ApiEphemeraMessage = raw_message.sign(&keypair).unwrap();
    ///
    /// assert_eq!(signed_message.certificate.public_key, keypair.public_key().into());
    ///
    /// let bytes = raw_message.encode().unwrap();
    /// assert!(keypair.public_key().verify(&bytes, &signed_message.certificate.signature.into()));
    /// ```
    ///
    /// # Errors
    /// - If the message can't be encoded.
    /// - If the message can't be signed.
    pub fn sign(&self, keypair: &Keypair) -> anyhow::Result<ApiEphemeraMessage> {
        let certificate = Certificate::prepare(keypair, &self)?;
        let message = ApiEphemeraMessage::new(self.clone(), certificate.into());
        Ok(message)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiBlockHeader {
    /// The timestamp of the block. It's initialized when the block is created.
    /// It uses UTC time.
    pub timestamp: u64,
    /// The PeerId of the block producer instance.
    pub creator: PeerId,
    /// The height of the block.
    pub height: u64,
    /// The hash of the current block.
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiBlock {
    pub header: ApiBlockHeader,
    pub messages: Vec<ApiEphemeraMessage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiRawBlock {
    pub(crate) header: ApiBlockHeader,
    pub(crate) messages: Vec<ApiEphemeraMessage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiCertificate {
    pub signature: ApiSignature,
    pub public_key: ApiPublicKey,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiSignature(pub(crate) Signature);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiPublicKey(pub(crate) PublicKey);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiEphemeraConfig {
    /// The address of the node. It's the address what Ephemera instance uses to communicate with other nodes.
    pub protocol_address: String,
    /// The HTTP API address of the node.
    pub api_address: String,
    /// The WebSocket address of the node.
    pub websocket_address: String,
    /// Node's public key.
    ///
    /// # Converting to string and back example
    /// ```
    /// use ephemera::crypto::{EphemeraKeypair, Keypair, PublicKey};
    ///
    /// let keypair = Keypair::generate(None);
    /// let public_key = keypair.public_key().to_string();
    ///
    /// let from_str = public_key.parse::<PublicKey>().unwrap();
    ///
    /// assert_eq!(keypair.public_key(), from_str);
    /// ```
    pub public_key: String,
    /// True if the node is a block producer. It's a configuration option.
    pub block_producer: bool,
    /// The interval of block creation in seconds. It's a configuration option.
    pub block_creation_interval_sec: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiDhtQueryRequest {
    /// The key to query for in hex format.
    key: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiDhtQueryResponse {
    /// The key that was queried for in hex format.
    key: String,
    /// The value that was stored under the queried key in hex format.
    value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiHealth {
    pub(crate) status: HealthStatus,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiDhtStoreRequest {
    /// The key to store the value under in hex format.
    key: String,
    /// The value to store in hex format.
    value: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiBroadcastInfo {
    /// The PeerId of the local node.
    pub local_peer_id: PeerId,
    /// The list of the current members of the network.
    pub current_members: HashSet<PeerId>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiBlockBroadcastInfo {
    pub local_peer_id: PeerId,
    pub broadcast_group: Vec<PeerId>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct ApiVerifyMessageInBlock {
    pub block_hash: String,
    pub message_hash: String,
    pub message_index: usize,
}

impl ApiVerifyMessageInBlock {
    #[must_use]
    pub fn new(block_hash: String, message_hash: String, message_index: usize) -> Self {
        Self {
            block_hash,
            message_hash,
            message_index,
        }
    }
}

impl Display for ApiVerifyMessageInBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ block_hash: {}, message_hash: {} }}",
            self.block_hash, self.message_hash
        )
    }
}

impl ApiBlockBroadcastInfo {
    pub(crate) fn new(local_peer_id: PeerId, broadcast_group: Vec<PeerId>) -> Self {
        Self {
            local_peer_id,
            broadcast_group,
        }
    }
}

impl ApiBroadcastInfo {
    pub(crate) fn new(current_members: HashSet<PeerId>, local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id,
            current_members,
        }
    }
}

impl Display for ApiBroadcastInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let current_members = self.current_members.iter().map(ToString::to_string);
        write!(
            f,
            "{{ local_peer_id: {}, current_members: {current_members:?} }}",
            self.local_peer_id,
        )
    }
}

impl Display for ApiEphemeraMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ApiEphemeraMessage(timestamp: {}, label: {})",
            self.timestamp, self.label,
        )
    }
}

impl From<ApiEphemeraMessage> for RawApiEphemeraMessage {
    fn from(message: ApiEphemeraMessage) -> Self {
        RawApiEphemeraMessage {
            timestamp: message.timestamp,
            label: message.label,
            data: message.data,
        }
    }
}

impl From<ApiEphemeraMessage> for EphemeraMessage {
    fn from(message: ApiEphemeraMessage) -> Self {
        Self {
            timestamp: message.timestamp,
            label: message.label,
            data: message.data,
            certificate: message.certificate.into(),
        }
    }
}

impl Decode for RawApiEphemeraMessage {
    type Output = Self;

    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError> {
        Codec::decode(bytes)
    }
}

impl Encode for RawApiEphemeraMessage {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(self)
    }
}

impl Encode for &RawApiEphemeraMessage {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(self)
    }
}

impl Display for ApiBlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ApiBlockHeader(timestamp: {}, creator: {}, height: {}, hash: {})",
            self.timestamp, self.creator, self.height, self.hash,
        )
    }
}

impl ApiBlock {
    #[must_use]
    pub fn as_raw_block(&self) -> ApiRawBlock {
        ApiRawBlock {
            header: self.header.clone(),
            messages: self.messages.clone(),
        }
    }

    #[must_use]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    #[must_use]
    pub fn hash(&self) -> String {
        self.header.hash.clone()
    }

    /// # Errors
    /// - If the block is invalid.
    /// - If the block's certificate is invalid.
    /// - If the block's certificate is not signed by the block's creator.
    pub fn verify(&self, certificate: &ApiCertificate) -> Result<bool, ApiError> {
        let block: Block = self.clone().try_into()?;
        let valid = block.verify(&(certificate.clone()).into()).map_err(|e| {
            error!("Failed to verify block: {}", e);
            ApiError::Internal("Failed to verify block certificate".to_string())
        })?;
        Ok(valid)
    }
}

impl Display for ApiBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ApiBlock(header: {}, message_count: {})",
            self.header,
            self.message_count()
        )
    }
}

impl ApiRawBlock {
    pub fn new(header: ApiBlockHeader, messages: Vec<ApiEphemeraMessage>) -> Self {
        Self { header, messages }
    }
}

impl ApiCertificate {
    /// # Errors
    /// - `EncodingError` if the message cannot be encoded.
    /// - `KeyPairError` if the message cannot be signed.
    pub fn prepare<D: Encode>(key_pair: &Keypair, data: &D) -> anyhow::Result<Self> {
        Certificate::prepare(key_pair, data).map(Into::into)
    }

    /// # Errors
    /// -`EncodingError` if the message cannot be encoded.
    pub fn verify<D: Encode>(&self, data: &D) -> anyhow::Result<bool> {
        let certificate: Certificate = (self.clone()).into();
        Certificate::verify(&certificate, data)
    }
}

impl From<EphemeraMessage> for ApiEphemeraMessage {
    fn from(ephemera_message: EphemeraMessage) -> Self {
        Self {
            timestamp: ephemera_message.timestamp,
            label: ephemera_message.label,
            data: ephemera_message.data,
            certificate: ApiCertificate {
                signature: ephemera_message.certificate.signature.into(),
                public_key: ephemera_message.certificate.public_key.into(),
            },
        }
    }
}

impl From<Certificate> for ApiCertificate {
    fn from(signature: Certificate) -> Self {
        Self {
            signature: signature.signature.into(),
            public_key: signature.public_key.into(),
        }
    }
}

impl From<ApiCertificate> for Certificate {
    fn from(value: ApiCertificate) -> Self {
        Certificate {
            signature: value.signature.into(),
            public_key: value.public_key.into(),
        }
    }
}

impl From<&Block> for &ApiBlock {
    fn from(block: &Block) -> Self {
        let api_block: ApiBlock = block.clone().into();
        Box::leak(Box::new(api_block))
    }
}

impl From<Signature> for ApiSignature {
    fn from(signature: Signature) -> Self {
        Self(signature)
    }
}

impl From<ApiSignature> for Signature {
    fn from(signature: ApiSignature) -> Self {
        signature.0
    }
}

impl ApiPublicKey {
    pub fn peer_id(&self) -> String {
        self.0.peer_id().to_string()
    }
}

impl From<PublicKey> for ApiPublicKey {
    fn from(public_key: PublicKey) -> Self {
        Self(public_key)
    }
}

impl From<ApiPublicKey> for PublicKey {
    fn from(public_key: ApiPublicKey) -> Self {
        public_key.0
    }
}

impl From<Block> for ApiBlock {
    fn from(block: Block) -> Self {
        Self {
            header: ApiBlockHeader {
                timestamp: block.header.timestamp,
                creator: block.header.creator,
                height: block.header.height,
                hash: block.header.hash.to_string(),
            },
            messages: block.messages.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<ApiBlock> for Block {
    type Error = ApiError;

    fn try_from(api_block: ApiBlock) -> Result<Self, ApiError> {
        let messages: Vec<EphemeraMessage> = api_block
            .messages
            .into_iter()
            .map(Into::into)
            .collect::<Vec<EphemeraMessage>>();
        Ok(Self {
            header: BlockHeader {
                timestamp: api_block.header.timestamp,
                creator: api_block.header.creator,
                height: api_block.header.height,
                hash: api_block.header.hash.parse().map_err(|e| {
                    error!("Failed to parse block hash: {}", e);
                    ApiError::Internal("Failed to parse block hash".to_string())
                })?,
            },
            messages,
        })
    }
}

impl ApiDhtStoreRequest {
    #[must_use]
    pub fn new(key: &[u8], value: &[u8]) -> Self {
        let key = bytes2hex("0x", key);
        let value = bytes2hex("0x", value);
        Self { key, value }
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn key(&self) -> Vec<u8> {
        //We can unwrap here because the key is always valid.
        hex2bytes(&self.key).unwrap()
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn value(&self) -> Vec<u8> {
        //We can unwrap here because the value is always valid.
        hex2bytes(&self.value).unwrap()
    }
}

impl ApiDhtQueryRequest {
    #[must_use]
    pub fn new(key: &[u8]) -> Self {
        let key = bytes2hex("0x", key);
        Self { key }
    }

    #[must_use]
    pub fn key_encoded(&self) -> String {
        self.key.clone()
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn key(&self) -> Vec<u8> {
        //We can unwrap here because the value is always valid.
        hex2bytes(&self.key).unwrap()
    }

    pub(crate) fn parse_key(key: &str) -> Vec<u8> {
        hex2bytes(key).unwrap()
    }
}

impl ApiDhtQueryResponse {
    pub(crate) fn new(key: Vec<u8>, value: Vec<u8>) -> Self {
        let key = bytes2hex("0x", key);
        let value = bytes2hex("0x", value);
        Self { key, value }
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn key(&self) -> Vec<u8> {
        //We can unwrap here because the key is always valid.
        hex2bytes(&self.key).unwrap()
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn value(&self) -> Vec<u8> {
        //We can unwrap here because the value is always valid.
        hex2bytes(&self.value).unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::crypto::EphemeraKeypair;
    use crate::crypto::Keypair;

    use super::*;

    #[test]
    fn test_message_sign_ok() {
        let message_signing_keypair = Keypair::generate(None);

        let message = RawApiEphemeraMessage::new("test".to_string(), vec![1, 2, 3]);
        let signed_message = message
            .sign(&message_signing_keypair)
            .expect("Failed to sign message");

        let certificate = signed_message.certificate;

        assert!(certificate.verify(&message).unwrap());
    }

    #[test]
    fn test_message_sign_fail() {
        let message_signing_keypair = Keypair::generate(None);

        let message = RawApiEphemeraMessage::new("test1".to_string(), vec![1, 2, 3]);
        let signed_message = message
            .sign(&message_signing_keypair)
            .expect("Failed to sign message");

        let certificate = signed_message.certificate;

        let modified_message = RawApiEphemeraMessage::new("test2".to_string(), vec![1, 2, 3]);
        assert!(!certificate.verify(&modified_message).unwrap());
    }
}
