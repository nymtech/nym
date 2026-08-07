// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{Addr, Storage};
use cw_controllers::Admin;
use cw_storage_plus::Item;
use nym_geolocation_contract_common::constants::storage_keys;
use nym_geolocation_contract_common::GeolocationContractError;
use nym_lthash::LtHash16;

pub const GEOLOCATION_CONTRACT_STORAGE: GeolocationStorage = GeolocationStorage::new();

pub struct GeolocationStorage {
    /// Admin of the contract; gates privileged operations.
    pub(crate) contract_admin: Admin,

    /// Address of the mixnet contract; used to verify a node id refers to a
    /// real, registered, and bonded node.
    pub(crate) mixnet_contract_address: Item<Addr>,
}

impl GeolocationStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> GeolocationStorage {
        GeolocationStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT_ADDRESS),
        }
    }

    /// Load the global LtHash accumulator, or the empty digest if nothing has been
    /// written yet.
    pub(crate) fn load_digest(
        &self,
        store: &dyn Storage,
    ) -> Result<LtHash16, GeolocationContractError> {
        match store.get(storage_keys::DIGEST_STATE.as_bytes()) {
            Some(bytes) => {
                let raw: &[u8; nym_lthash::DIGEST_LEN] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| GeolocationContractError::CorruptDigestState)?;
                Ok(LtHash16::from_bytes(raw))
            }
            None => Ok(LtHash16::new()),
        }
    }

    fn save_digest(&self, store: &mut dyn Storage, digest: &LtHash16) {
        store.set(storage_keys::DIGEST_STATE.as_bytes(), &digest.to_bytes());
    }
}
