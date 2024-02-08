// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_credential_storage::models::{StorableIssuedCredential, StoredIssuedCredential};
use nym_credentials::coconut::bandwidth::IssuedBandwidthCredential;
use nym_validator_client::nym_api::EpochId;

pub fn stored_credential_to_issued_bandwidth(
    cred: StoredIssuedCredential,
) -> Result<IssuedBandwidthCredential, BandwidthControllerError> {
    /*
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

    */
    todo!()
}

pub fn issued_bandwidth_to_stored_credential(
    issued: IssuedBandwidthCredential,
    epoch_id: EpochId,
) -> StoredIssuedCredential {
    todo!()
}
