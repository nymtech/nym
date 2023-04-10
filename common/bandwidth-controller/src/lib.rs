// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;

use nym_credential_storage::error::StorageError;
use nym_credential_storage::storage::Storage;

use std::str::FromStr;
use {
    nym_coconut_interface::Base58,
    nym_credentials::coconut::{
        bandwidth::prepare_for_spending, utils::obtain_aggregate_verification_key,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use nym_validator_client::nyxd::traits::DkgQueryClient;

#[cfg(target_arch = "wasm32")]
use crate::wasm_mockups::DkgQueryClient;

#[cfg(not(target_arch = "wasm32"))]
pub mod acquire;
pub mod error;
#[cfg(target_arch = "wasm32")]
pub mod wasm_mockups;

pub struct BandwidthController<C, St: Storage> {
    storage: St,
    client: C,
}

impl<C, St: Storage> BandwidthController<C, St> {
    pub fn new(storage: St, client: C) -> Self {
        BandwidthController { storage, client }
    }

    pub fn storage(&self) -> &St {
        &self.storage
    }

    pub async fn prepare_coconut_credential(
        &self,
    ) -> Result<(nym_coconut_interface::Credential, i64), BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
    {
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
        let coconut_api_clients =
            nym_validator_client::CoconutApiClient::all_coconut_api_clients(&self.client, epoch_id)
                .await?;
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

    pub async fn consume_credential(&self, id: i64) -> Result<(), BandwidthControllerError> {
        // JS: shouldn't we send some contract/validator/gateway message here to actually, you know,
        // consume it?
        Ok(self.storage.consume_coconut_credential(id).await?)
    }
}

impl<C, St> Clone for BandwidthController<C, St>
where
    C: Clone,
    St: Storage + Clone,
{
    fn clone(&self) -> Self {
        BandwidthController {
            storage: self.storage.clone(),
            client: self.client.clone(),
        }
    }
}
