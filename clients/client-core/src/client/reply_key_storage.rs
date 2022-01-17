// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use nymsphinx::anonymous_replies::{
    encryption_key::EncryptionKeyDigest, encryption_key::Unsigned, SurbEncryptionKey,
    SurbEncryptionKeySize,
};
use std::path::Path;

#[derive(Debug)]
pub enum ReplyKeyStorageError {
    DbReadError(sled::Error),
    DbWriteError(sled::Error),
    DbOpenError(sled::Error),
}

/// Permanent storage for keys in all sent [`ReplySURB`]
///
/// Each sent out [`ReplySURB`] has a new key associated with it that is going to be used for
/// payload encryption. In order to -decrypt whatever reply we receive, we need to know which
/// key to use for that purpose. We do it based on received `H(t)` which has to be included
/// with each reply.
/// Moreover, there is no restriction when the [`ReplySURB`] might get used so we need to
/// have a permanent storage for all the keys that we might ever see in the future.
#[derive(Debug, Clone)]
pub struct ReplyKeyStorage {
    db: sled::Db,
}

impl ReplyKeyStorage {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ReplyKeyStorageError> {
        let db = match sled::open(path) {
            Err(e) => return Err(ReplyKeyStorageError::DbOpenError(e)),
            Ok(db) => db,
        };

        Ok(ReplyKeyStorage { db })
    }

    fn read_encryption_key(&self, raw_key: sled::IVec) -> SurbEncryptionKey {
        let key_bytes_ref = raw_key.as_ref();
        // if this fails it means we have some database corruption and we
        // absolutely can't continue

        if key_bytes_ref.len() != SurbEncryptionKeySize::to_usize() {
            error!("REPLY KEY STORAGE DATA CORRUPTION - ENCRYPTION KEY HAS INVALID LENGTH");
            panic!("REPLY KEY STORAGE DATA CORRUPTION - ENCRYPTION KEY HAS INVALID LENGTH");
        }

        // this can only fail if the bytes have invalid length but we already asserted it
        SurbEncryptionKey::try_from_bytes(key_bytes_ref).unwrap()
    }

    // TOOD: perhaps we could also store some part of original message here too?
    pub fn insert_encryption_key(
        &mut self,
        encryption_key: SurbEncryptionKey,
    ) -> Result<(), ReplyKeyStorageError> {
        let digest = encryption_key.compute_digest();

        let insertion_result = match self.db.insert(digest, encryption_key.to_bytes()) {
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

    // Once we use key once, we do not expect to use it again
    pub fn get_and_remove_encryption_key(
        &self,
        key_digest: EncryptionKeyDigest,
    ) -> Result<Option<SurbEncryptionKey>, ReplyKeyStorageError> {
        let removal_result = match self.db.remove(key_digest) {
            Err(e) => Err(ReplyKeyStorageError::DbReadError(e)),
            Ok(existing_key) => {
                Ok(existing_key.map(|existing_key| self.read_encryption_key(existing_key)))
            }
        };

        // TODO: not sure how to feel about flushing it every single time here...
        // same with insertion
        self.db.flush().unwrap();
        removal_result
    }
}
