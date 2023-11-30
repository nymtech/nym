// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_credential_storage::error::StorageError;
use nym_credential_storage::storage::Storage;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use std::str::FromStr;
use zeroize::Zeroizing;
use {
    nym_coconut_interface::Base58,
    nym_credentials::coconut::{
        bandwidth::prepare_for_spending, utils::obtain_aggregate_verification_key,
    },
};

pub mod acquire;
pub mod error;

pub struct BandwidthController<C, St> {
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
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let bandwidth_credential = self
            .storage
            .get_next_coconut_credential()
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?;
        let voucher_value = u64::from_str(&bandwidth_credential.voucher_value)
            .map_err(|_| StorageError::InconsistentData)?;
        let voucher_info = bandwidth_credential.voucher_info.clone();
        let serial_number = Zeroizing::new(nym_coconut_interface::Attribute::try_from_bs58(
            bandwidth_credential.serial_number,
        )?);
        let binding_number = Zeroizing::new(nym_coconut_interface::Attribute::try_from_bs58(
            bandwidth_credential.binding_number,
        )?);
        let signature =
            nym_coconut_interface::Signature::try_from_bs58(bandwidth_credential.signature)?;
        let epoch_id = u64::from_str(&bandwidth_credential.epoch_id)
            .map_err(|_| StorageError::InconsistentData)?;

        let coconut_api_clients = all_coconut_api_clients(&self.client, epoch_id).await?;

        let verification_key = obtain_aggregate_verification_key(&coconut_api_clients).await?;

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        Ok((
            prepare_for_spending(
                voucher_value,
                voucher_info,
                &serial_number,
                &binding_number,
                epoch_id,
                &signature,
                &verification_key,
            )?,
            bandwidth_credential.id,
        ))
    }

    pub async fn consume_credential(&self, id: i64) -> Result<(), BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        // JS: shouldn't we send some contract/validator/gateway message here to actually, you know,
        // consume it?
        self.storage
            .consume_coconut_credential(id)
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
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
