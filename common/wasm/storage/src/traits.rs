// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::WasmStorage;
use async_trait::async_trait;
use indexed_db_futures::primitive::{TryFromJs, TryToJs};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;
use wasm_bindgen::JsValue;

#[async_trait(?Send)]
pub trait BaseWasmStorage {
    type StorageError: Error;

    async fn exists(db_name: &str) -> Result<bool, Self::StorageError>;

    async fn read_value<T, K>(&self, store: &str, key: K) -> Result<Option<T>, Self::StorageError>
    where
        T: DeserializeOwned,
        K: TryToJs;

    async fn store_value<T, K>(
        &self,
        store: &str,
        key: K,
        value: &T,
    ) -> Result<(), Self::StorageError>
    where
        T: Serialize,
        K: TryToJs + TryFromJs;

    async fn remove_value<K>(&self, store: &str, key: K) -> Result<(), Self::StorageError>
    where
        K: TryToJs;

    async fn has_value<K>(&self, store: &str, key: K) -> Result<bool, Self::StorageError>
    where
        K: TryToJs;

    async fn key_count<K>(&self, store: &str, key: K) -> Result<u32, Self::StorageError>
    where
        K: TryToJs;

    async fn get_all_keys(&self, store: &str) -> Result<Vec<JsValue>, Self::StorageError>;
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
        K: TryToJs,
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
        K: TryToJs + TryFromJs,
    {
        self.store_value(store, key, value).await
    }

    async fn remove_value<K>(&self, store: &str, key: K) -> Result<(), Self::StorageError>
    where
        K: TryToJs,
    {
        self.remove_value(store, key).await
    }

    async fn has_value<K>(&self, store: &str, key: K) -> Result<bool, Self::StorageError>
    where
        K: TryToJs,
    {
        self.has_value(store, key).await
    }

    async fn key_count<K>(&self, store: &str, key: K) -> Result<u32, Self::StorageError>
    where
        K: TryToJs,
    {
        self.key_count(store, key).await
    }

    async fn get_all_keys(&self, store: &str) -> Result<Vec<JsValue>, Self::StorageError> {
        self.get_all_keys(store).await
    }
}
