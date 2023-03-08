// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;

#[cfg(target_arch = "wasm32")]
use crate::wasm_mockups::Storage;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "mobile"))]
use nym_credential_storage::storage::Storage;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "mobile")]
use mobile_storage::Storage;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "mobile")]
use mobile_storage::StorageError;

#[cfg(target_arch = "wasm32")]
use crate::wasm_mockups::StorageError;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "mobile"))]
use nym_credential_storage::error::StorageError;

#[cfg(target_arch = "wasm32")]
use crate::wasm_mockups::{Client, CosmWasmClient};
use std::str::FromStr;
#[cfg(not(target_arch = "wasm32"))]
use validator_client::{nyxd::CosmWasmClient, Client};
use {
    nym_coconut_interface::Base58,
    nym_credentials::coconut::{
        bandwidth::prepare_for_spending, utils::obtain_aggregate_verification_key,
    },
};

// TODO: make it nicer for wasm (I don't want to touch it for this experiment)
#[cfg(target_arch = "wasm32")]
use crate::wasm_mockups::PersistentStorage;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "mobile"))]
use nym_credential_storage::PersistentStorage;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "mobile")]
use mobile_storage::PersistentStorage;

#[derive(Clone)]
#[allow(dead_code)]
pub struct BandwidthController<C: Clone, St: Storage = PersistentStorage> {
    storage: St,
    nyxd_client: Client<C>,
}

impl<C, St> BandwidthController<C, St>
where
    C: CosmWasmClient + Sync + Send + Clone,
    St: Storage + Clone + 'static,
{
    pub fn new(storage: St, nyxd_client: Client<C>) -> Self {
        BandwidthController {
            storage,
            nyxd_client,
        }
    }

    pub async fn prepare_coconut_credential(
        &self,
    ) -> Result<(nym_coconut_interface::Credential, i64), GatewayClientError> {
        let bandwidth_credential = self.storage.get_next_coconut_credential().await?;
        let voucher_value = u64::from_str(&bandwidth_credential.voucher_value)
            .map_err(|_| StorageError::InconsistentData)?;
        let voucher_info = bandwidth_credential.voucher_info.clone();
        let serial_number =
            nym_coconut_interface::Attribute::try_from_bs58(bandwidth_credential.serial_number)?;
        let binding_number =
            nym_coconut_interface::Attribute::try_from_bs58(bandwidth_credential.binding_number)?;
        let signature =
            nym_coconut_interface::Signature::try_from_bs58(bandwidth_credential.signature)?;
        let epoch_id = u64::from_str(&bandwidth_credential.epoch_id)
            .map_err(|_| StorageError::InconsistentData)?;

        #[cfg(not(target_arch = "wasm32"))]
        let coconut_api_clients = validator_client::CoconutApiClient::all_coconut_api_clients(
            &self.nyxd_client,
            epoch_id,
        )
        .await
        .expect("Could not query api clients");
        #[cfg(target_arch = "wasm32")]
        let coconut_api_clients = vec![];
        let verification_key = obtain_aggregate_verification_key(&coconut_api_clients).await?;

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        Ok((
            prepare_for_spending(
                voucher_value,
                voucher_info,
                serial_number,
                binding_number,
                epoch_id,
                &signature,
                &verification_key,
            )?,
            bandwidth_credential.id,
        ))
    }

    pub async fn consume_credential(&self, id: i64) -> Result<(), GatewayClientError> {
        Ok(self.storage.consume_coconut_credential(id).await?)
    }
}
