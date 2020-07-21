// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use log::*;
use nymsphinx::anonymous_replies::{
    encryption_key::Unsigned,
    encryption_key::{DefaultHasher, EncryptionKeyDigest},
    SURBEncryptionKey, SURBEncryptionKeySize,
};
use std::path::Path;

#[derive(Debug)]
pub(crate) enum ReplyKeyStorageError {
    DbReadError(sled::Error),
    DbWriteError(sled::Error),
    DbOpenError(sled::Error),
}

/// Permanent storage for keys in all sent [`ReplySURB`]
///
/// Each sent out [`ReplySURB`] has a new key associated with it that is going to be used for
/// payload encryption. In order to decrypt whatever reply we receive, we need to know which
/// key to use for that purpose. We do it based on received `H(t)` which has to be included
/// with each reply.
/// Moreover, there is no restriction when the [`ReplySURB`] might get used so we need to
/// have a permanent storage for all the keys that we might ever see in the future.
#[derive(Debug, Clone)]
pub struct ReplyKeyStorage {
    db: sled::Db,
}

impl ReplyKeyStorage {
    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Result<Self, ReplyKeyStorageError> {
        let db = match sled::open(path) {
            Err(e) => return Err(ReplyKeyStorageError::DbOpenError(e)),
            Ok(db) => db,
        };

        Ok(ReplyKeyStorage { db })
    }

    fn read_encryption_key(&self, raw_key: sled::IVec) -> SURBEncryptionKey {
        let key_bytes_ref = raw_key.as_ref();
        // if this fails it means we have some database corruption and we
        // absolutely can't continue

        if key_bytes_ref.len() != SURBEncryptionKeySize::to_usize() {
            error!("REPLY KEY STORAGE DATA CORRUPTION - ENCRYPTION KEY HAS INVALID LENGTH");
            panic!("REPLY KEY STORAGE DATA CORRUPTION - ENCRYPTION KEY HAS INVALID LENGTH");
        }

        // this can only fail if the bytes have invalid length but we already asserted it
        SURBEncryptionKey::try_from_bytes(key_bytes_ref).unwrap()
    }

    // TOOD: perhaps we could also store some part of original message here too?
    pub(crate) fn insert_encryption_key(
        &mut self,
        encryption_key: SURBEncryptionKey,
    ) -> Result<(), ReplyKeyStorageError> {
        let digest = encryption_key.compute_digest::<DefaultHasher>();

        let insertion_result = match self.db.insert(digest.to_vec(), encryption_key.to_bytes()) {
            Err(e) => Err(ReplyKeyStorageError::DbWriteError(e)),
            Ok(existing_key) => {
                if existing_key.is_some() {
                    panic!("HASH COLLISION DETECTED")
                };
                Ok(())
            }
        };

        // TODO: perhaps we could implement some batching mechanism to avoid frequent flushes?
        self.db.flush().unwrap();
        insertion_result
    }

    // perhaps this is not going to be useful? because once we get a key we do not expect to
    // ever need it again
    // pub(crate) fn get_encryption_key(
    //     &self,
    //     key_digest: EncryptionKeyDigest,
    // ) -> Result<Option<SURBEncryptionKey>, ReplyKeyStorageError> {
    //     match self.db.get(&key_digest) {
    //         Err(e) => Err(ReplyKeyStorageError::DbReadError(e)),
    //         Ok(existing_key) => Ok(existing_key.map(|key_ivec| self.read_encryption_key(key_ivec))),
    //     }
    // }

    // Once we use key once, we do not expect to use it again
    pub(crate) fn get_and_remove_encryption_key(
        &self,
        key_digest: EncryptionKeyDigest,
    ) -> Result<Option<SURBEncryptionKey>, ReplyKeyStorageError> {
        let removal_result = match self.db.remove(&key_digest.to_vec()) {
            Err(e) => Err(ReplyKeyStorageError::DbReadError(e)),
            Ok(existing_key) => {
                Ok(existing_key.map(|existing_key| self.read_encryption_key(existing_key)))
            }
        };

        // removal of keys happens extremely rarely, so flush is also fine here
        self.db.flush().unwrap();
        removal_result
    }
}
