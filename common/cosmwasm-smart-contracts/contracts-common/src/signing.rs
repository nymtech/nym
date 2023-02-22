// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{from_slice, to_vec, Addr, Coin, MessageInfo, StdResult};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub type Nonce = u32;

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageType(String);

impl<T> From<T> for MessageType
where
    T: ToString,
{
    fn from(value: T) -> Self {
        MessageType(value.to_string())
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SigningAlgorithm {
    #[default]
    Ed25519,
    Secp256k1,
}

// TODO: maybe move this one to repo-wide common?
#[derive(Serialize, Deserialize)]
pub struct SignableMessage<T> {
    nonce: u32,
    algorithm: SigningAlgorithm,
    content: T,
}

impl<T> SignableMessage<T> {
    pub fn new(nonce: u32, content: T) -> Self {
        SignableMessage {
            nonce,
            algorithm: SigningAlgorithm::Ed25519,
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

    pub fn to_string(&self) -> StdResult<String>
    where
        T: Serialize,
    {
        // if you look into implementation of `serde_json_wasm::to_string` this [i.e. the String conversion]
        // CAN'T fail, but let's avoid this unnecessary unwrap either way
        self.to_plaintext()
            .map(|s| String::from_utf8(s).unwrap_or(String::from("SERIALIZATION FAILURE")))
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
}

#[derive(Serialize)]
struct ContractMessageContent<T> {
    message_type: MessageType,
    signer: Addr,
    proxy: Option<Addr>,
    funds: Vec<Coin>,
    data: T,
}

impl<T> ContractMessageContent<T> {
    pub fn new(
        message_type: MessageType,
        signer: Addr,
        proxy: Option<Addr>,
        funds: Vec<Coin>,
        data: T,
    ) -> Self {
        ContractMessageContent {
            message_type,
            signer,
            proxy,
            funds,
            data,
        }
    }

    pub fn new_with_info(
        message_type: MessageType,
        info: MessageInfo,
        signer: Addr,
        data: T,
    ) -> Self {
        let proxy = if info.sender == signer {
            None
        } else {
            Some(info.sender)
        };

        ContractMessageContent {
            message_type,
            signer,
            proxy,
            funds: info.funds,
            data,
        }
    }
}
