// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_store_cipher::ExportedStoreCipher;
use serde::{Deserialize, Serialize};

// we can't store `Option<ExportedStoreCipher>` directly since a `None` is converted into js' `undefined`
// which is equivalent of having no value at all.
// instead we want to know if initial account was created with no encryption so we wouldn't overwrite anything.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StoredExportedStoreCipher {
    NoEncryption,
    Cipher(ExportedStoreCipher),
}

impl StoredExportedStoreCipher {
    pub(crate) fn uses_encryption(&self) -> bool {
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
