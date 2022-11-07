// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;

#[cfg(target_arch = "wasm32")]
use crate::wasm_storage::Storage;
#[cfg(not(target_arch = "wasm32"))]
use credential_storage::storage::Storage;

#[cfg(all(target_arch = "wasm32", feature = "coconut"))]
use crate::wasm_storage::StorageError;

#[cfg(all(not(target_arch = "wasm32"), feature = "coconut"))]
use credential_storage::error::StorageError;

#[cfg(feature = "coconut")]
use std::str::FromStr;
#[cfg(feature = "coconut")]
use {
    coconut_interface::Base58,
    credentials::coconut::{
        bandwidth::prepare_for_spending, utils::obtain_aggregate_verification_key,
    },
};

#[derive(Clone)]
pub struct BandwidthController<St: Storage> {
    #[allow(dead_code)]
    storage: St,
    #[cfg(feature = "coconut")]
    validator_endpoints: Vec<url::Url>,
}

impl<St> BandwidthController<St>
where
    St: Storage + Clone + 'static,
{
    #[cfg(feature = "coconut")]
    pub fn new(storage: St, validator_endpoints: Vec<url::Url>) -> Self {
        BandwidthController {
            storage,
            validator_endpoints,
        }
    }

    #[cfg(not(feature = "coconut"))]
    pub fn new(storage: St) -> Result<Self, GatewayClientError> {
        Ok(BandwidthController { storage })
    }

    #[cfg(feature = "coconut")]
    pub async fn prepare_coconut_credential(
        &self,
    ) -> Result<coconut_interface::Credential, GatewayClientError> {
        let verification_key = obtain_aggregate_verification_key(&self.validator_endpoints).await?;
        let bandwidth_credential = self.storage.get_next_coconut_credential().await?;
        let voucher_value = u64::from_str(&bandwidth_credential.voucher_value)
            .map_err(|_| StorageError::InconsistentData)?;
        let voucher_info = bandwidth_credential.voucher_info.clone();
        let serial_number =
            coconut_interface::Attribute::try_from_bs58(bandwidth_credential.serial_number)?;
        let binding_number =
            coconut_interface::Attribute::try_from_bs58(bandwidth_credential.binding_number)?;
        let signature =
            coconut_interface::Signature::try_from_bs58(bandwidth_credential.signature)?;

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)
        Ok(prepare_for_spending(
            voucher_value,
            voucher_info,
            serial_number,
            binding_number,
            &signature,
            &verification_key,
        )?)
    }
}
