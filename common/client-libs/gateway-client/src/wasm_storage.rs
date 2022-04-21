// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Wasm client is not yet supported")]
    WasmNotSupported,

    #[allow(dead_code)]
    #[error("Code shouldn't reach this point")]
    InconsistentData,
}

#[derive(Clone)]
pub struct PersistentStorage {}

pub struct CoconutCredential {
    pub id: i64,
    pub voucher_value: String,
    pub voucher_info: String,
    pub serial_number: String,
    pub binding_number: String,
    pub signature: String,
}

pub struct ERC20Credential {
    pub id: i64,
    pub public_key: String,
    pub private_key: String,
    pub consumed: bool,
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
    ) -> Result<(), StorageError>;

    async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, StorageError>;

    async fn remove_coconut_credential(&self, id: i64) -> Result<(), StorageError>;

    async fn insert_erc20_credential(
        &self,
        public_key: String,
        private_key: String,
    ) -> Result<(), StorageError>;

    async fn get_next_erc20_credential(&self) -> Result<ERC20Credential, StorageError>;

    async fn consume_erc20_credential(&self, public_key: String) -> Result<(), StorageError>;
}

#[async_trait]
impl Storage for PersistentStorage {
    async fn insert_coconut_credential(
        &self,
        _voucher_value: String,
        _voucher_info: String,
        _serial_number: String,
        _binding_number: String,
        _signature: String,
    ) -> Result<(), StorageError> {
        Err(StorageError::WasmNotSupported)
    }

    async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, StorageError> {
        Err(StorageError::WasmNotSupported)
    }

    async fn remove_coconut_credential(&self, _id: i64) -> Result<(), StorageError> {
        Err(StorageError::WasmNotSupported)
    }

    async fn insert_erc20_credential(
        &self,
        _public_key: String,
        _private_key: String,
    ) -> Result<(), StorageError> {
        Err(StorageError::WasmNotSupported)
    }

    async fn get_next_erc20_credential(&self) -> Result<ERC20Credential, StorageError> {
        Err(StorageError::WasmNotSupported)
    }

    async fn consume_erc20_credential(&self, _public_key: String) -> Result<(), StorageError> {
        Err(StorageError::WasmNotSupported)
    }
}
