// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_mock::ContractMock;
use base64::{engine::general_purpose, Engine};
use cosmwasm_std::Env;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::num::ParseIntError;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodingError {
    #[error("failed to parse the block height information of the state dump: {source}")]
    MalformedBlockHeight {
        #[from]
        source: ParseIntError,
    },

    #[error("failed to open the specified state dump: {source}")]
    FileOpenError {
        #[from]
        source: io::Error,
    },

    #[error("failed to decode the provided json state: {source}")]
    JsonDecodeError {
        #[from]
        source: serde_json::Error,
    },

    #[error("failed to decode one of the state keys: {source}")]
    HexDecodeError {
        #[from]
        source: hex::FromHexError,
    },

    #[error("failed to decode one of the state values: {source}")]
    Base64DecodeError {
        #[from]
        source: base64::DecodeError,
    },
}

#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("failed to open the specified state dump file: {source}")]
    FileOpenError {
        #[from]
        source: io::Error,
    },

    #[error("failed to encode the provided json state: {source}")]
    JsonEncodeError {
        #[from]
        source: serde_json::Error,
    },
}

pub struct ContractState {
    pub height: u64,
    pub data: Vec<KeyValue>,
}

pub struct KeyValue {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl ContractState {
    pub fn try_from_json(value: &str) -> Result<Self, DecodingError> {
        RawContractState::from_json(value)?.decode()
    }

    pub fn try_load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, DecodingError> {
        RawContractState::from_file(path)?.decode()
    }

    pub fn find_value(&self, key: &[u8]) -> Option<&[u8]> {
        self.data
            .iter()
            .find(|kv| kv.key == key)
            .map(|kv| kv.value.as_ref())
    }

    pub fn into_test_mock(self, custom_env: Option<Env>) -> ContractMock {
        ContractMock::from_state_dump(self, custom_env)
    }

    pub(crate) fn encode(self) -> RawContractState {
        RawContractState {
            height: self.height.to_string(),
            result: self.data.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RawContractState {
    height: String,
    result: Vec<RawKeyValue>,
}

impl RawContractState {
    fn decode(self) -> Result<ContractState, DecodingError> {
        Ok(ContractState {
            height: self.height.parse()?,
            data: self
                .result
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }

    fn from_json(value: &str) -> Result<Self, DecodingError> {
        Ok(serde_json::from_str(value)?)
    }

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DecodingError> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }

    pub(crate) fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), EncodingError> {
        let file = File::open(path)?;
        Ok(serde_json::to_writer(file, &self)?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RawKeyValue {
    // encoded as hex
    key: String,

    // encoded as base64
    value: String,
}

impl TryFrom<RawKeyValue> for KeyValue {
    type Error = DecodingError;

    fn try_from(raw: RawKeyValue) -> Result<Self, Self::Error> {
        Ok(KeyValue {
            key: hex::decode(&raw.key)?,
            value: general_purpose::STANDARD.decode(&raw.value)?,
        })
    }
}

impl From<KeyValue> for RawKeyValue {
    fn from(decoded: KeyValue) -> Self {
        RawKeyValue {
            key: hex::encode(decoded.key),
            value: general_purpose::STANDARD.encode(decoded.value),
        }
    }
}
