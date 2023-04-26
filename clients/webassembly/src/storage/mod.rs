// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use indexed_db_futures::prelude::*;
use indexed_db_futures::web_sys::DomException;
use js_sys::Promise;
use nym_client_core::client::key_manager::{KeyManager, KeyStore};
use nym_crypto::asymmetric::{encryption, identity};
use nym_store_cipher::{Aes256Gcm, EncryptedData, ExportedStoreCipher, KdfInfo, StoreCipher};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, console_warn, simple_js_error, PromisableResult};
use zeroize::Zeroizing;

const STORAGE_NAME_PREFIX: &str = "wasm-client-storage";
const STORAGE_VERSION: u32 = 1;

// v1 tables
mod v1 {
    // stores
    pub const KEYS_STORE: &str = "keys";
    pub const CORE_STORE: &str = "core";

    // keys
    pub const CIPHER_STORE_EXPORT: &str = "cipher_store_export_info";

    pub const ED25519_IDENTITY_KEYPAIR: &str = "ed25519_identity_keypair";
    pub const X25519_ENCRYPTION_KEYPAIR: &str = "x25519_encryption_key";

    // TODO: for those we could actually use the subtle crypto storage
    pub const AES128CTR_ACK_KEY: &str = "aes128ctr_ack_key";
    pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS_PREFIX: &str =
        "aes128ctr_blake3_hmac_gateway_keys";
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Json(#[from] serde_wasm_bindgen::Error),

    #[error("DomException {name} ({code}): {message}")]
    DomException {
        /// DomException code
        code: u16,
        /// Specific name of the DomException
        name: String,
        /// Message given to the DomException
        message: String,
    },

    #[error("encountered issue with our storage encryption layer: {source}")]
    CryptoStorageError {
        #[from]
        source: nym_store_cipher::Error,
    },

    #[error(
        "attempted to unlock an existing encrypted client store without providing a passphrase"
    )]
    NoPassphraseProvided,

    #[error("attempted to access an existing unencrypted client store with a passphrase")]
    UnexpectedPassphraseProvided,
}

impl From<StorageError> for JsValue {
    fn from(value: StorageError) -> Self {
        simple_js_error(value.to_string())
    }
}

impl From<DomException> for StorageError {
    fn from(value: DomException) -> StorageError {
        StorageError::DomException {
            name: value.name(),
            message: value.message(),
            code: value.code(),
        }
    }
}

// we can't store `Option<ExportedStoreCipher>` directly since a `None` is converted into js' `undefined`
// which is equivalent of having no value at all.
// instead we want to know if initial account was created with no encryption so we wouldn't overwrite anything.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredExportedStoreCipher {
    NoEncryption,
    Cipher(ExportedStoreCipher),
}

impl StoredExportedStoreCipher {
    fn uses_encryption(&self) -> bool {
        matches!(self, StoredExportedStoreCipher::Cipher(..))
    }
}

impl From<Option<ExportedStoreCipher>> for StoredExportedStoreCipher {
    fn from(value: Option<ExportedStoreCipher>) -> Self {
        match value {
            None => StoredExportedStoreCipher::NoEncryption,
            Some(exported) => StoredExportedStoreCipher::Cipher(exported),
        }
    }
}

#[wasm_bindgen]
pub struct ClientStorage {
    pub(crate) name: String,
    pub(crate) inner: Arc<ClientStorageInner>,
}

#[wasm_bindgen]
impl ClientStorage {
    fn db_name(client_id: &str) -> String {
        format!("{STORAGE_NAME_PREFIX}-{client_id}")
    }

    async fn new_async(client_id: &str, passphrase: Option<String>) -> Result<Self, StorageError> {
        let name = Self::db_name(client_id);

        // make sure the password is zeroized when no longer used, especially if we error out.
        // special care must be taken on JS side to ensure it's correctly used there.
        let passphrase = Zeroizing::new(passphrase);

        Ok(ClientStorage {
            inner: Arc::new(
                ClientStorageInner::new(&name, passphrase.as_ref().map(|p| p.as_bytes())).await?,
            ),
            name,
        })
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(client_id: String, passphrase: String) -> Promise {
        future_to_promise(async move {
            Self::new_async(&client_id, Some(passphrase))
                .await
                .into_promise_result()
        })
    }

    pub fn new_unencrypted(client_id: String) -> Promise {
        future_to_promise(async move {
            Self::new_async(&client_id, None)
                .await
                .into_promise_result()
        })
    }
}

pub(crate) struct ClientStorageInner {
    pub(crate) inner: IdbDatabase,
    store_cipher: Option<StoreCipher>,
}

impl ClientStorageInner {
    async fn new(db_name: &str, passphrase: Option<&[u8]>) -> Result<Self, StorageError> {
        let mut db_req: OpenDbRequest = IdbDatabase::open_u32(db_name, STORAGE_VERSION)?;

        db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
            // Even if the web-sys bindings expose the version as a f64, the IndexedDB API
            // works with an unsigned integer.
            // See <https://github.com/rustwasm/wasm-bindgen/issues/1149>
            let old_version = evt.old_version() as u32;

            if old_version < 1 {
                // migrating to version 1
                let db = evt.db();

                db.create_object_store(v1::KEYS_STORE)?;
                db.create_object_store(v1::CORE_STORE)?;
            }

            Ok(())
        }));

        let db: IdbDatabase = db_req.into_future().await?;
        let existing_cipher_info = Self::read_exported_cipher_store(&db).await?;

        // TODO: clean up this match...
        let store_cipher = match passphrase {
            None => {
                console_log!("no encryption will be used for this client storage");
                if let Some(prior_client_data) = existing_cipher_info {
                    if prior_client_data.uses_encryption() {
                        console_warn!("BAD  1");
                        return Err(StorageError::NoPassphraseProvided);
                    }
                }
                // if this was a `None`, it's good, it means that client hasn't been initialised before
                Self::store_exported_cipher_store(&db, StoredExportedStoreCipher::NoEncryption)
                    .await?;
                None
            }
            Some(passphrase) => {
                if let Some(prior_client_data) = existing_cipher_info {
                    if let StoredExportedStoreCipher::Cipher(exported_cipher) = prior_client_data {
                        console_log!("attempting to use previously derived encryption key");
                        Some(StoreCipher::import_aes256gcm(passphrase, exported_cipher)?)
                    } else {
                        console_warn!("BAD  2");
                        return Err(StorageError::UnexpectedPassphraseProvided);
                    }
                } else {
                    console_log!("attempting to derive new encryption key");
                    let store_cipher = StoreCipher::<Aes256Gcm>::new_with_default_kdf(passphrase)?;
                    let exported = store_cipher.export_aes256gcm()?;
                    Self::store_exported_cipher_store(&db, Some(exported).into()).await?;

                    Some(store_cipher)
                }
            }
        };

        Ok(ClientStorageInner {
            inner: db,
            store_cipher,
        })
    }

    // I really dislike the signature on this method, but how to refactor this?
    async fn read_exported_cipher_store(
        db: &IdbDatabase,
    ) -> Result<Option<StoredExportedStoreCipher>, StorageError> {
        db.transaction_on_one_with_mode(v1::CORE_STORE, IdbTransactionMode::Readonly)?
            .object_store(v1::CORE_STORE)?
            .get(&JsValue::from_str(v1::CIPHER_STORE_EXPORT))?
            .await?
            .map(serde_wasm_bindgen::from_value)
            .transpose()
            .map_err(Into::into)
    }

    // I really dislike the signature on this method, but how to refactor this?
    async fn store_exported_cipher_store(
        db: &IdbDatabase,
        exported_store_cipher: StoredExportedStoreCipher,
    ) -> Result<(), StorageError> {
        db.transaction_on_one_with_mode(v1::CORE_STORE, IdbTransactionMode::Readwrite)?
            .object_store(v1::CORE_STORE)?
            .put_key_val_owned(
                &JsValue::from_str(v1::CIPHER_STORE_EXPORT),
                &serde_wasm_bindgen::to_value(&exported_store_cipher)?,
            )?
            .into_future()
            .await
            .map_err(Into::into)
    }

    fn serialize_value<T: Serialize>(&self, value: &T) -> Result<JsValue, StorageError> {
        if let Some(cipher) = &self.store_cipher {
            let encrypted = cipher.encrypt_json_value(value)?;
            Ok(serde_wasm_bindgen::to_value(&encrypted)?)
        } else {
            Ok(serde_wasm_bindgen::to_value(&value)?)
        }
    }

    fn deserialize_value<T: DeserializeOwned>(&self, value: JsValue) -> Result<T, StorageError> {
        if let Some(cipher) = &self.store_cipher {
            let encrypted: EncryptedData = serde_wasm_bindgen::from_value(value)?;
            Ok(cipher.decrypt_json_value(encrypted)?)
        } else {
            Ok(serde_wasm_bindgen::from_value(value)?)
        }
    }

    async fn read_value<T, K>(&self, store: &str, key: K) -> Result<Option<T>, StorageError>
    where
        T: DeserializeOwned,
        K: wasm_bindgen::JsCast,
    {
        self.inner
            .transaction_on_one_with_mode(store, IdbTransactionMode::Readonly)?
            .object_store(store)?
            .get(&key)?
            .await?
            .map(|raw| self.deserialize_value(raw))
            .transpose()
    }

    async fn store_value<T, K>(&self, store: &str, key: K, value: &T) -> Result<(), StorageError>
    where
        T: Serialize,
        K: wasm_bindgen::JsCast,
    {
        self.inner
            .transaction_on_one_with_mode(store, IdbTransactionMode::Readwrite)?
            .object_store(store)?
            .put_key_val_owned(key, &self.serialize_value(&value)?)?
            .into_future()
            .await
            .map_err(Into::into)
    }

    async fn read_identity_keypair(&self) -> Result<Option<identity::KeyPair>, StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
        )
        .await
    }

    async fn read_encryption_keypair(&self) -> Result<Option<encryption::KeyPair>, StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
        )
        .await
    }

    async fn store_identity_keypair(
        &self,
        keypair: &identity::KeyPair,
    ) -> Result<(), StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
            keypair,
        )
        .await
    }

    async fn store_encryption_keypair(
        &self,
        keypair: &encryption::KeyPair,
    ) -> Result<(), StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
            keypair,
        )
        .await
    }
}

impl Drop for ClientStorageInner {
    fn drop(&mut self) {
        // Must release the database access manually as it's not done when
        // dropping it.
        self.inner.close();
    }
}

#[async_trait(?Send)]
impl KeyStore for ClientStorage {
    type StorageError = StorageError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        todo!()
    }

    async fn store_keys(&self, keys: KeyManager) -> Result<(), Self::StorageError> {
        todo!()
    }
}
