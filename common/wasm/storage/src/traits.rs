// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::WasmStorage;
use async_trait::async_trait;
use js_sys::Array;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;

#[async_trait(?Send)]
pub trait BaseWasmStorage {
    type StorageError: Error;

    async fn exists(db_name: &str) -> Result<bool, Self::StorageError>;

    async fn read_value<T, K>(&self, store: &str, key: K) -> Result<Option<T>, Self::StorageError>
    where
        T: DeserializeOwned,
        K: wasm_bindgen::JsCast;

    async fn store_value<T, K>(
        &self,
        store: &str,
        key: K,
        value: &T,
    ) -> Result<(), Self::StorageError>
    where
        T: Serialize,
        K: wasm_bindgen::JsCast;

    async fn remove_value<K>(&self, store: &str, key: K) -> Result<(), Self::StorageError>
    where
        K: wasm_bindgen::JsCast;

    async fn has_value<K>(&self, store: &str, key: K) -> Result<bool, Self::StorageError>
    where
        K: wasm_bindgen::JsCast;

    async fn key_count<K>(&self, store: &str, key: K) -> Result<u32, Self::StorageError>
    where
        K: wasm_bindgen::JsCast;

    async fn get_all_keys(&self, store: &str) -> Result<js_sys::Array, Self::StorageError>;
}

#[async_trait(?Send)]
impl BaseWasmStorage for WasmStorage {
    type StorageError = crate::error::StorageError;

    async fn exists(db_name: &str) -> Result<bool, Self::StorageError> {
        WasmStorage::exists(db_name).await
    }

    async fn read_value<T, K>(&self, store: &str, key: K) -> Result<Option<T>, Self::StorageError>
    where
        T: DeserializeOwned,
        K: wasm_bindgen::JsCast,
    {
        self.read_value(store, key).await
    }

    async fn store_value<T, K>(
        &self,
        store: &str,
        key: K,
        value: &T,
    ) -> Result<(), Self::StorageError>
    where
        T: Serialize,
        K: wasm_bindgen::JsCast,
    {
        self.store_value(store, key, value).await
    }

    async fn remove_value<K>(&self, store: &str, key: K) -> Result<(), Self::StorageError>
    where
        K: wasm_bindgen::JsCast,
    {
        self.remove_value(store, key).await
    }

    async fn has_value<K>(&self, store: &str, key: K) -> Result<bool, Self::StorageError>
    where
        K: wasm_bindgen::JsCast,
    {
        self.has_value(store, key).await
    }

    async fn key_count<K>(&self, store: &str, key: K) -> Result<u32, Self::StorageError>
    where
        K: wasm_bindgen::JsCast,
    {
        self.key_count(store, key).await
    }

    async fn get_all_keys(&self, store: &str) -> Result<Array, Self::StorageError> {
        self.get_all_keys(store).await
    }
}
