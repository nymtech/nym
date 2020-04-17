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

use directory_client::presence::providers::MixProviderClient;
use log::*;
use sfw_provider_requests::auth_token::{AuthToken, AUTH_TOKEN_SIZE};
use sphinx::constants::DESTINATION_ADDRESS_LENGTH;
use sphinx::route::DestinationAddressBytes;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum ClientLedgerError {
    DbReadError(sled::Error),
    DbWriteError(sled::Error),
    DbOpenError(sled::Error),
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
            Err(e) => return Err(ClientLedgerError::DbOpenError(e)),
            Ok(db) => db,
        };

        let ledger = ClientLedger { db };

        ledger.db.iter().keys().for_each(|key| {
            println!(
                "key: {:?}",
                ledger
                    .read_destination_address_bytes(key.unwrap())
                    .to_base58_string()
            );
        });

        Ok(ledger)
    }

    fn read_auth_token(&self, raw_token: sled::IVec) -> AuthToken {
        let token_bytes_ref = raw_token.as_ref();
        // if this fails it means we have some database corruption and we
        // absolutely can't continue
        if token_bytes_ref.len() != AUTH_TOKEN_SIZE {
            error!("CLIENT LEDGER DATA CORRUPTION - TOKEN HAS INVALID LENGTH");
            panic!("CLIENT LEDGER DATA CORRUPTION - TOKEN HAS INVALID LENGTH");
        }

        let mut token_bytes = [0u8; AUTH_TOKEN_SIZE];
        token_bytes.copy_from_slice(token_bytes_ref);
        AuthToken::from_bytes(token_bytes)
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

    pub(crate) fn verify_token(
        &self,
        auth_token: &AuthToken,
        client_address: &DestinationAddressBytes,
    ) -> Result<bool, ClientLedgerError> {
        match self.db.get(&client_address.to_bytes()) {
            Err(e) => Err(ClientLedgerError::DbReadError(e)),
            Ok(token) => match token {
                Some(token_ivec) => Ok(&self.read_auth_token(token_ivec) == auth_token),
                None => Ok(false),
            },
        }
    }

    pub(crate) fn insert_token(
        &mut self,
        auth_token: AuthToken,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<AuthToken>, ClientLedgerError> {
        let insertion_result = match self
            .db
            .insert(&client_address.to_bytes(), &auth_token.to_bytes())
        {
            Err(e) => Err(ClientLedgerError::DbWriteError(e)),
            Ok(existing_token) => {
                Ok(existing_token.map(|existing_token| self.read_auth_token(existing_token)))
            }
        };

        // registration doesn't happen that often so might as well flush it to the disk to be sure
        self.db.flush().unwrap();
        insertion_result
    }

    pub(crate) fn remove_token(
        &mut self,
        client_address: &DestinationAddressBytes,
    ) -> Result<Option<AuthToken>, ClientLedgerError> {
        let removal_result = match self.db.remove(&client_address.to_bytes()) {
            Err(e) => Err(ClientLedgerError::DbWriteError(e)),
            Ok(existing_token) => {
                Ok(existing_token.map(|existing_token| self.read_auth_token(existing_token)))
            }
        };

        // removing of tokens happens extremely rarely, so flush is also fine here
        self.db.flush().unwrap();
        removal_result
    }

    pub(crate) fn current_clients(&self) -> Result<Vec<MixProviderClient>, ClientLedgerError> {
        let clients = self.db.iter().keys();

        let mut client_vec = Vec::new();
        for client in clients {
            match client {
                Err(e) => return Err(ClientLedgerError::DbWriteError(e)),
                Ok(client_entry) => client_vec.push(MixProviderClient {
                    pub_key: self
                        .read_destination_address_bytes(client_entry)
                        .to_base58_string(),
                }),
            }
        }

        Ok(client_vec)
    }

    #[cfg(test)]
    pub(crate) fn create_temporary() -> Self {
        let cfg = sled::Config::new().temporary(true);
        ClientLedger {
            db: cfg.open().unwrap(),
        }
    }
}
