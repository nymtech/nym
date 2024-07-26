// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{from_slice, to_vec, Addr, Coin, MessageInfo, StdResult};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
pub use verifier::Verifier;

pub mod verifier;

pub type Nonce = u32;

// define this type explicitly for [hopefully] better usability
// (so you wouldn't need to worry about whether you should use bytes, bs58, etc.)
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MessageSignature(Vec<u8>);

impl MessageSignature {
    pub fn as_bs58_string(&self) -> String {
        bs58::encode(&self.0).into_string()
    }
}

impl<'a> From<&'a [u8]> for MessageSignature {
    fn from(value: &'a [u8]) -> Self {
        MessageSignature(value.to_vec())
    }
}

impl From<Vec<u8>> for MessageSignature {
    fn from(value: Vec<u8>) -> Self {
        MessageSignature(value)
    }
}

impl<'a> TryFrom<&'a str> for MessageSignature {
    type Error = bs58::decode::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Ok(MessageSignature(bs58::decode(value).into_vec()?))
    }
}

impl TryFrom<String> for MessageSignature {
    type Error = bs58::decode::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl AsRef<[u8]> for MessageSignature {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for MessageSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = String::deserialize(deserializer)?;
        let bytes = bs58::decode(inner).into_vec().map_err(de::Error::custom)?;
        Ok(MessageSignature(bytes))
    }
}

impl Serialize for MessageSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bs58_encoded = self.as_bs58_string();
        bs58_encoded.serialize(serializer)
    }
}

impl Display for MessageSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_bs58_string())
    }
}

impl FromStr for MessageSignature {
    type Err = bs58::decode::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

pub trait SigningPurpose {
    fn message_type() -> MessageType;
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageType(String);

impl MessageType {
    pub fn new<S: Into<String>>(typ: S) -> Self {
        MessageType(typ.into())
    }
}

impl<T> From<T> for MessageType
where
    T: ToString,
{
    fn from(value: T) -> Self {
        MessageType(value.to_string())
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SigningAlgorithm {
    #[default]
    Ed25519,
    Secp256k1,
}

impl SigningAlgorithm {
    pub fn is_ed25519(&self) -> bool {
        matches!(self, SigningAlgorithm::Ed25519)
    }
}

// TODO: maybe move this one to repo-wide common?
// TODO: should it perhaps also include the public key itself?
#[derive(Serialize, Deserialize)]
pub struct SignableMessage<T> {
    pub nonce: u32,
    pub algorithm: SigningAlgorithm,
    pub message_type: MessageType,

    pub content: T,
}

impl<T> SignableMessage<T>
where
    T: SigningPurpose,
{
    pub fn new(nonce: u32, content: T) -> Self {
        SignableMessage {
            nonce,
            algorithm: SigningAlgorithm::Ed25519,
            message_type: T::message_type(),
            content,
        }
    }

    pub fn with_signing_algorithm(mut self, algorithm: SigningAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    pub fn to_plaintext(&self) -> StdResult<Vec<u8>>
    where
        T: Serialize,
    {
        to_vec(self)
    }

    pub fn to_sha256_plaintext_digest(&self) -> StdResult<Vec<u8>>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    pub fn to_json_string(&self) -> StdResult<String>
    where
        T: Serialize,
    {
        // if you look into implementation of `serde_json_wasm::to_string` this [i.e. the String conversion]
        // CAN'T fail, but let's avoid this unnecessary unwrap either way
        self.to_plaintext()
            .map(|s| String::from_utf8(s).unwrap_or(String::from("SERIALIZATION FAILURE")))
    }

    pub fn to_base58_string(&self) -> StdResult<String>
    where
        T: Serialize,
    {
        self.to_plaintext().map(|s| bs58::encode(s).into_string())
    }

    pub fn try_from_bytes(bytes: &[u8]) -> StdResult<SignableMessage<T>>
    where
        T: DeserializeOwned,
    {
        from_slice(bytes)
    }

    pub fn try_from_string(raw: &str) -> StdResult<SignableMessage<T>>
    where
        T: DeserializeOwned,
    {
        Self::try_from_bytes(raw.as_bytes())
    }

    pub fn try_from_base58_string(raw: &str) -> bs58::decode::Result<StdResult<SignableMessage<T>>>
    where
        T: DeserializeOwned,
    {
        bs58::decode(raw)
            .into_vec()
            .map(|d| Self::try_from_bytes(&d))
    }
}

#[derive(Serialize)]
pub struct ContractMessageContent<T> {
    pub sender: Addr,
    pub funds: Vec<Coin>,
    pub data: T,
}

impl<T> SigningPurpose for ContractMessageContent<T>
where
    T: SigningPurpose,
{
    fn message_type() -> MessageType {
        T::message_type()
    }
}

impl<T> ContractMessageContent<T> {
    pub fn new(sender: Addr, funds: Vec<Coin>, data: T) -> Self {
        ContractMessageContent {
            sender,
            funds,
            data,
        }
    }

    pub fn new_with_info(info: MessageInfo, signer: Addr, data: T) -> Self {
        ContractMessageContent {
            sender: signer,
            funds: info.funds,
            data,
        }
    }
}
