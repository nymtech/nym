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

use crate::config::persistence::key_pathfinder::ClientKeyPathfinder;
use crypto::asymmetric::{encryption, identity};
use gateway_requests::registration::handshake::SharedKey;
use log::*;
use nymsphinx::acknowledgements::AckAes128Key;
use rand::{CryptoRng, RngCore};
use std::io;
use std::sync::Arc;

// Note: to support key rotation in the future, all keys will require adding an extra smart pointer,
// most likely an AtomicCell, or if it doesn't work as I think it does, a Mutex. Although I think
// AtomicCell includes a Mutex implicitly if the underlying type does not work atomically.
// And I guess there will need to be some mechanism for a grace period when you can still
// use the old key after new one was issued.
pub(crate) struct KeyManager {
    /// identity key associated with the client instance.
    identity_keypair: Arc<identity::KeyPair>,
    /// encryption key associated with the client instance.
    encryption_keypair: Arc<encryption::KeyPair>,
    /// shared key derived with the gateway during "registration handshake"
    gateway_shared_key: Option<Arc<SharedKey>>,
    /// key used for producing and processing acknowledgement packets.
    ack_key: Arc<AckAes128Key>,
}

// The expected flow of a KeyManager "lifetime" is as follows:
/*
   1. ::new() is called during client-init
   2. after gateway registration is completed [in init] ::insert_gateway_shared_key() is called
   3. ::store_keys() is called before init finishes execution.
   4. ::load_keys() is called at the beginning of each subsequent client-run
   5. [not implemented] ::rotate_keys() is called periodically during client-run I presume?
*/

impl KeyManager {
    /// Creates new instance of a [`KeyManager`]
    pub(crate) fn new<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        KeyManager {
            identity_keypair: Arc::new(identity::KeyPair::new_with_rng(rng)),
            encryption_keypair: Arc::new(encryption::KeyPair::new_with_rng(rng)),
            gateway_shared_key: None,
            ack_key: Arc::new(AckAes128Key::new(rng)),
        }
    }

    /// After shared key with the gateway is derived, puts its ownership to this instance of a [`KeyManager`].
    pub(crate) fn insert_gateway_shared_key(&mut self, gateway_shared_key: SharedKey) {
        self.gateway_shared_key = Some(Arc::new(gateway_shared_key))
    }

    /// Loads previously stored keys from the disk.
    pub(crate) fn load_keys(client_pathfinder: &ClientKeyPathfinder) -> io::Result<Self> {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                client_pathfinder.private_identity_key().to_owned(),
                client_pathfinder.public_identity_key().to_owned(),
            ))?;
        let encryption_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                client_pathfinder.private_encryption_key().to_owned(),
                client_pathfinder.public_encryption_key().to_owned(),
            ))?;

        let gateway_shared_key: SharedKey =
            pemstore::load_key(&client_pathfinder.gateway_shared_key().to_owned())?;

        let ack_key: AckAes128Key = pemstore::load_key(&client_pathfinder.ack_key().to_owned())?;

        // TODO: ack key is never stored so it is generated now. But perhaps it should be stored
        // after all for consistency sake?
        Ok(KeyManager {
            identity_keypair: Arc::new(identity_keypair),
            encryption_keypair: Arc::new(encryption_keypair),
            gateway_shared_key: Some(Arc::new(gateway_shared_key)),
            ack_key: Arc::new(ack_key),
        })
    }

    /// Stores all available keys on the disk.
    // While perhaps there is no much point in storing the `AckAes128Key` on the disk,
    // it is done so for the consistency sake so that you wouldn't require an rng instance
    // during `load_keys` to generate the said key.
    pub(crate) fn store_keys(&self, client_pathfinder: &ClientKeyPathfinder) -> io::Result<()> {
        pemstore::store_keypair(
            self.identity_keypair.as_ref(),
            &pemstore::KeyPairPath::new(
                client_pathfinder.private_identity_key().to_owned(),
                client_pathfinder.public_identity_key().to_owned(),
            ),
        )?;
        pemstore::store_keypair(
            self.encryption_keypair.as_ref(),
            &pemstore::KeyPairPath::new(
                client_pathfinder.private_encryption_key().to_owned(),
                client_pathfinder.public_encryption_key().to_owned(),
            ),
        )?;

        pemstore::store_key(self.ack_key.as_ref(), &client_pathfinder.ack_key())?;

        match self.gateway_shared_key.as_ref() {
            None => warn!("No gateway shared key available to store!"),
            Some(gate_key) => {
                pemstore::store_key(gate_key.as_ref(), &client_pathfinder.gateway_shared_key())?
            }
        }

        Ok(())
    }

    /// Gets an atomically reference counted pointer to [`identity::KeyPair`].
    pub(crate) fn identity_keypair(&self) -> Arc<identity::KeyPair> {
        Arc::clone(&self.identity_keypair)
    }

    /// Gets an atomically reference counted pointer to [`encryption::KeyPair`].
    pub(crate) fn encryption_keypair(&self) -> Arc<encryption::KeyPair> {
        Arc::clone(&self.encryption_keypair)
    }

    /// Gets an atomically reference counted pointer to [`SharedKey`].
    // since this function is not fully public, it is not expected to be used externally and
    // hence it's up to us to ensure it's called in correct context
    pub(crate) fn gateway_shared_key(&self) -> Arc<SharedKey> {
        Arc::clone(
            &self
                .gateway_shared_key
                .as_ref()
                .expect("tried to unwrap empty gateway key!"),
        )
    }

    /// Gets an atomically reference counted pointer to [`AckAes128Key`].
    pub(crate) fn ack_key(&self) -> Arc<AckAes128Key> {
        Arc::clone(&self.ack_key)
    }
}
