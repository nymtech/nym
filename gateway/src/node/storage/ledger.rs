// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::authentication::iv::AuthenticationIV;
use gateway_requests::generic_array::typenum::Unsigned;
use gateway_requests::registration::handshake::{SharedKeySize, SharedKeys};
use log::*;
use nymsphinx::{DestinationAddressBytes, DESTINATION_ADDRESS_LENGTH};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum ClientLedgerError {
    Read(sled::Error),
    Write(sled::Error),
    Open(sled::Error),
}

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
pub(crate) struct ClientLedger {
    db: sled::Db,
}

impl ClientLedger {
    pub(crate) fn load(file: PathBuf) -> Result<Self, ClientLedgerError> {
        let db = match sled::open(file) {
            Err(e) => return Err(ClientLedgerError::Open(e)),
            Ok(db) => db,
        };

        let ledger = ClientLedger { db };

        ledger.db.iter().keys().for_each(|key| {
            trace!(
                "key: {:?}",
                ledger
                    .read_destination_address_bytes(key.unwrap())
                    .to_base58_string()
            );
        });

        Ok(ledger)
    }

    fn read_shared_key(&self, raw_key: sled::IVec) -> SharedKeys {
        let key_bytes_ref = raw_key.as_ref();
        // if this fails it means we have some database corruption and we
        // absolutely can't continue

        if key_bytes_ref.len() != SharedKeySize::to_usize() {
            error!("CLIENT LEDGER DATA CORRUPTION - SHARED KEY HAS INVALID LENGTH");
            panic!("CLIENT LEDGER DATA CORRUPTION - SHARED KEY HAS INVALID LENGTH");
        }

        // this can only fail if the bytes have invalid length but we already asserted it
        SharedKeys::try_from_bytes(key_bytes_ref).unwrap()
    }

    fn read_destination_address_bytes(
        &self,
        raw_destination: sled::IVec,
    ) -> DestinationAddressBytes {
        let destination_ref = raw_destination.as_ref();
        // if this fails it means we have some database corruption and we
        // absolutely can't continue
        if destination_ref.len() != DESTINATION_ADDRESS_LENGTH {
            error!("CLIENT LEDGER DATA CORRUPTION - CLIENT ADDRESS HAS INVALID LENGTH");
            panic!("CLIENT LEDGER DATA CORRUPTION - CLIENT ADDRESS HAS INVALID LENGTH");
        }

        let mut destination_bytes = [0u8; DESTINATION_ADDRESS_LENGTH];
        destination_bytes.copy_from_slice(destination_ref);
        DestinationAddressBytes::from_bytes(destination_bytes)
    }

    pub(crate) fn verify_shared_key(
        &self,
        client_address: &DestinationAddressBytes,
        encrypted_address: &EncryptedAddressBytes,
        iv: &AuthenticationIV,
    ) -> Result<bool, ClientLedgerError> {
        match self.db.get(&client_address.to_bytes()) {
            Err(e) => Err(ClientLedgerError::Read(e)),
            Ok(existing_key) => match existing_key {
                Some(existing_key_ivec) => {
                    let shared_key = &self.read_shared_key(existing_key_ivec);
                    Ok(encrypted_address.verify(client_address, shared_key, iv))
                }
                None => Ok(false),
            },
        }
    }

    pub(crate) fn get_shared_key(
        &self,
        client_address: &DestinationAddressBytes,
    ) -> Result<Option<SharedKeys>, ClientLedgerError> {
        match self.db.get(&client_address.to_bytes()) {
            Err(e) => Err(ClientLedgerError::Read(e)),
            Ok(existing_key) => Ok(existing_key.map(|key_ivec| self.read_shared_key(key_ivec))),
        }
    }

    pub(crate) fn insert_shared_key(
        &mut self,
        shared_key: SharedKeys,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<SharedKeys>, ClientLedgerError> {
        let insertion_result = match self
            .db
            .insert(&client_address.to_bytes(), shared_key.to_bytes())
        {
            Err(e) => Err(ClientLedgerError::Write(e)),
            Ok(existing_key) => {
                Ok(existing_key.map(|existing_key| self.read_shared_key(existing_key)))
            }
        };

        // registration doesn't happen that often so might as well flush it to the disk to be sure
        self.db.flush().unwrap();
        insertion_result
    }

    pub(crate) fn remove_shared_key(
        &mut self,
        client_address: &DestinationAddressBytes,
    ) -> Result<Option<SharedKeys>, ClientLedgerError> {
        let removal_result = match self.db.remove(&client_address.to_bytes()) {
            Err(e) => Err(ClientLedgerError::Write(e)),
            Ok(existing_key) => {
                Ok(existing_key.map(|existing_key| self.read_shared_key(existing_key)))
            }
        };

        // removal of keys happens extremely rarely, so flush is also fine here
        self.db.flush().unwrap();
        removal_result
    }
}
