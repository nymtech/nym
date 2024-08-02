// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::helpers::IssuedExpirationDateSignatures;
use crate::ecash::storage::helpers::{
    deserialise_coin_index_signatures, deserialise_expiration_date_signatures,
    serialise_coin_index_signatures, serialise_expiration_date_signatures,
};
use crate::ecash::storage::manager::EcashStorageManagerExt;
use crate::ecash::storage::models::{
    join_attributes, EpochCredentials, IssuedTicketbook, SerialNumberWrapper, TicketProvider,
};
use crate::node_status_api::models::NymApiStorageError;
use crate::support::storage::NymApiStorage;
use nym_api_requests::ecash::models::Pagination;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::BlindedSignature;
use nym_compact_ecash::VerificationKeyAuth;
use nym_config::defaults::BloomfilterParameters;
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::identity;
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::nyxd::AccountId;
use time::{Date, OffsetDateTime};

mod helpers;
pub(crate) mod manager;
pub(crate) mod models;

const DEFAULT_CREDENTIALS_PAGE_LIMIT: u32 = 100;

#[async_trait]
pub trait EcashStorageExt {
    async fn get_double_spending_filter_params(
        &self,
    ) -> Result<(i64, BloomfilterParameters), NymApiStorageError>;

    async fn update_archived_partial_bloomfilter(
        &self,
        date: Date,
        new_bitmap: &[u8],
    ) -> Result<(), NymApiStorageError>;

    async fn try_load_partial_bloomfilter_bitmap(
        &self,
        date: Date,
        params_id: i64,
    ) -> Result<Option<Vec<u8>>, NymApiStorageError>;

    async fn insert_partial_bloomfilter(
        &self,
        date: Date,
        params_id: i64,
        bitmap: &[u8],
    ) -> Result<(), NymApiStorageError>;

    async fn remove_old_partial_bloomfilters(&self, cutoff: Date)
        -> Result<(), NymApiStorageError>;

    async fn remove_expired_verified_tickets(&self, cutoff: Date)
        -> Result<(), NymApiStorageError>;

    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, NymApiStorageError>;

    #[allow(dead_code)]
    async fn create_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
    ) -> Result<(), NymApiStorageError>;

    async fn update_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
        credential_id: i64,
    ) -> Result<(), NymApiStorageError>;

    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedTicketbook>, NymApiStorageError>;

    async fn get_issued_bandwidth_credential_by_deposit_id(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<IssuedTicketbook>, NymApiStorageError>;

    #[allow(clippy::too_many_arguments)]
    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        deposit_id: DepositId,
        partial_credential: &BlindedSignature,
        signature: identity::Signature,
        private_commitments: Vec<Vec<u8>>,
        expiration_date: Date,
        ticketbook_type: TicketType,
    ) -> Result<i64, NymApiStorageError>;

    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedTicketbook>, NymApiStorageError>;

    async fn get_issued_credentials_paged(
        &self,
        pagination: Pagination<i64>,
    ) -> Result<Vec<IssuedTicketbook>, NymApiStorageError>;
    //
    // async fn insert_credential(
    //     &self,
    //     credential: &CredentialSpendingData,
    //     serial_number_bs58: String,
    //     gateway_addr: &AccountId,
    //     proposal_id: u64,
    // ) -> Result<(), NymApiStorageError>;
    //
    async fn get_credential_data(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<CredentialSpendingData>, NymApiStorageError>;

    async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<(), NymApiStorageError>;

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, NymApiStorageError>;

    async fn get_verified_tickets_since(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError>;

    async fn get_all_spent_tickets_on(
        &self,
        date: Date,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError>;

    async fn get_or_create_ticket_provider_with_id(
        &self,
        gateway_address: &str,
    ) -> Result<i64, NymApiStorageError>;

    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<VerificationKeyAuth>, NymApiStorageError>;
    async fn insert_master_verification_key(
        &self,
        epoch_id: EpochId,
        key: &VerificationKeyAuth,
    ) -> Result<(), NymApiStorageError>;

    async fn get_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError>;
    async fn insert_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError>;

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError>;
    async fn insert_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError>;

    async fn get_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError>;
    async fn insert_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError>;

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError>;
    async fn insert_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError>;
}

#[async_trait]
impl EcashStorageExt for NymApiStorage {
    async fn get_double_spending_filter_params(
        &self,
    ) -> Result<(i64, BloomfilterParameters), NymApiStorageError> {
        match self
            .manager
            .get_latest_double_spending_filter_params()
            .await?
        {
            Some(raw) => Ok((raw.id, (&raw).try_into()?)),
            None => {
                let default = BloomfilterParameters::default_ecash();
                info!("using default bloomfilter parameters: {default:?}");
                let id = self
                    .manager
                    .insert_double_spending_filter_params(
                        default.num_hashes,
                        default.bitmap_size as u32,
                        &default.sip_keys[0].0.to_be_bytes(),
                        &default.sip_keys[0].1.to_be_bytes(),
                        &default.sip_keys[1].0.to_be_bytes(),
                        &default.sip_keys[1].1.to_be_bytes(),
                    )
                    .await?;
                Ok((id, default))
            }
        }
    }

    async fn update_archived_partial_bloomfilter(
        &self,
        date: Date,
        new_bitmap: &[u8],
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .update_archived_partial_bloomfilter(date, new_bitmap)
            .await?)
    }

    async fn try_load_partial_bloomfilter_bitmap(
        &self,
        date: Date,
        params_id: i64,
    ) -> Result<Option<Vec<u8>>, NymApiStorageError> {
        Ok(self
            .manager
            .try_load_partial_bloomfilter_bitmap(date, params_id)
            .await?)
    }

    async fn insert_partial_bloomfilter(
        &self,
        date: Date,
        params_id: i64,
        bitmap: &[u8],
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .insert_partial_bloomfilter(date, params_id, bitmap)
            .await?)
    }

    async fn remove_old_partial_bloomfilters(
        &self,
        cutoff: Date,
    ) -> Result<(), NymApiStorageError> {
        Ok(self.manager.remove_old_partial_bloomfilters(cutoff).await?)
    }

    async fn remove_expired_verified_tickets(
        &self,
        cutoff: Date,
    ) -> Result<(), NymApiStorageError> {
        Ok(self.manager.remove_expired_verified_tickets(cutoff).await?)
    }

    async fn get_epoch_credentials(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochCredentials>, NymApiStorageError> {
        Ok(self.manager.get_epoch_credentials(epoch_id).await?)
    }

    async fn create_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .create_epoch_credentials_entry(epoch_id)
            .await?)
    }

    async fn update_epoch_credentials_entry(
        &self,
        epoch_id: EpochId,
        credential_id: i64,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .update_epoch_credentials_entry(epoch_id, credential_id)
            .await?)
    }

    async fn get_issued_credential(
        &self,
        credential_id: i64,
    ) -> Result<Option<IssuedTicketbook>, NymApiStorageError> {
        Ok(self.manager.get_issued_credential(credential_id).await?)
    }

    async fn get_issued_bandwidth_credential_by_deposit_id(
        &self,
        deposit_id: DepositId,
    ) -> Result<Option<IssuedTicketbook>, NymApiStorageError> {
        Ok(self
            .manager
            .get_issued_bandwidth_credential_by_deposit_id(deposit_id)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    async fn store_issued_credential(
        &self,
        epoch_id: u32,
        deposit_id: DepositId,
        partial_credential: &BlindedSignature,
        signature: identity::Signature,
        private_commitments: Vec<Vec<u8>>,
        expiration_date: Date,
        ticketbook_type: TicketType,
    ) -> Result<i64, NymApiStorageError> {
        Ok(self
            .manager
            .store_issued_ticketbook(
                epoch_id,
                deposit_id,
                &partial_credential.to_bytes(),
                &signature.to_bytes(),
                &join_attributes(private_commitments),
                expiration_date,
                ticketbook_type.encode(),
            )
            .await?)
    }

    async fn get_issued_credentials(
        &self,
        credential_ids: Vec<i64>,
    ) -> Result<Vec<IssuedTicketbook>, NymApiStorageError> {
        Ok(self.manager.get_issued_ticketbooks(credential_ids).await?)
    }

    async fn get_issued_credentials_paged(
        &self,
        pagination: Pagination<i64>,
    ) -> Result<Vec<IssuedTicketbook>, NymApiStorageError> {
        // rows start at 1
        let start_after = pagination.last_key.unwrap_or(0);
        let limit = match pagination.limit {
            Some(v) => {
                if v == 0 || v > DEFAULT_CREDENTIALS_PAGE_LIMIT {
                    DEFAULT_CREDENTIALS_PAGE_LIMIT
                } else {
                    v
                }
            }
            None => DEFAULT_CREDENTIALS_PAGE_LIMIT,
        };

        Ok(self
            .manager
            .get_issued_ticketbooks_paged(start_after, limit)
            .await?)
    }

    // async fn insert_credential(
    //     &self,
    //     credential: &CredentialSpendingData,
    //     serial_number_bs58: String,
    //     gateway_addr: &AccountId,
    //     proposal_id: u64,
    // ) -> Result<(), NymApiStorageError> {
    //     self.manager
    //         .insert_credential(
    //             credential.to_bs58(),
    //             serial_number_bs58,
    //             gateway_addr.to_string(),
    //             proposal_id as i64,
    //         )
    //         .await
    //         .map_err(|err| err.into())
    // }

    async fn get_credential_data(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<CredentialSpendingData>, NymApiStorageError> {
        let ticket = self.manager.get_ticket(serial_number).await?;
        ticket
            .map(|ticket| {
                CredentialSpendingData::try_from_bytes(&ticket.ticket_data).map_err(|_| {
                    NymApiStorageError::DatabaseInconsistency {
                        reason: "impossible to deserialize verified ticket".to_string(),
                    }
                })
            })
            .transpose()
    }

    async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<(), NymApiStorageError> {
        let provider_id = self
            .get_or_create_ticket_provider_with_id(gateway_addr.as_ref())
            .await?;

        let now = OffsetDateTime::now_utc();

        let ticket_bytes = ticket_data.to_bytes();
        let encoded_serial_number = ticket_data.encoded_serial_number();
        self.manager
            .insert_verified_ticket(
                provider_id,
                ticket_data.spend_date,
                now,
                ticket_bytes,
                encoded_serial_number,
            )
            .await
            .map_err(Into::into)
    }

    async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>, NymApiStorageError> {
        self.manager
            .get_ticket_provider(gateway_address)
            .await
            .map_err(Into::into)
    }

    async fn get_verified_tickets_since(
        &self,
        provider_id: i64,
        since: OffsetDateTime,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError> {
        self.manager
            .get_provider_ticket_serial_numbers(provider_id, since)
            .await
            .map_err(Into::into)
    }

    async fn get_all_spent_tickets_on(
        &self,
        date: Date,
    ) -> Result<Vec<SerialNumberWrapper>, NymApiStorageError> {
        self.manager
            .get_spent_tickets_on(date)
            .await
            .map_err(Into::into)
    }

    async fn get_or_create_ticket_provider_with_id(
        &self,
        gateway_address: &str,
    ) -> Result<i64, NymApiStorageError> {
        if let Some(provider) = self.get_ticket_provider(gateway_address).await? {
            Ok(provider.id)
        } else {
            Ok(self.manager.insert_ticket_provider(gateway_address).await?)
        }
    }

    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<VerificationKeyAuth>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_master_verification_key(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let master_vk = VerificationKeyAuth::from_bytes(&raw).map_err(|_| {
            NymApiStorageError::database_inconsistency("malformed stored master verification key")
        })?;

        Ok(Some(master_vk))
    }

    async fn insert_master_verification_key(
        &self,
        epoch_id: EpochId,
        key: &VerificationKeyAuth,
    ) -> Result<(), NymApiStorageError> {
        Ok(self
            .manager
            .insert_master_verification_key(epoch_id as i64, &key.to_bytes())
            .await?)
    }

    async fn get_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_partial_coin_index_signatures(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(deserialise_coin_index_signatures(&raw)?))
    }

    async fn insert_partial_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_partial_coin_index_signatures(
                epoch_id as i64,
                &serialise_coin_index_signatures(sigs),
            )
            .await?;
        Ok(())
    }

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_master_coin_index_signatures(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(deserialise_coin_index_signatures(&raw)?))
    }

    async fn insert_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        sigs: &[AnnotatedCoinIndexSignature],
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_master_coin_index_signatures(
                epoch_id as i64,
                &serialise_coin_index_signatures(sigs),
            )
            .await?;
        Ok(())
    }

    async fn get_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_partial_expiration_date_signatures(expiration_date)
            .await?
        else {
            return Ok(None);
        };

        let signatures = deserialise_expiration_date_signatures(&raw.serialised_signatures)?;

        Ok(Some(IssuedExpirationDateSignatures {
            epoch_id: raw.epoch_id as u64,
            signatures,
        }))
    }

    async fn insert_partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_partial_expiration_date_signatures(
                sigs.epoch_id as i64,
                expiration_date,
                &serialise_expiration_date_signatures(&sigs.signatures),
            )
            .await?;
        Ok(())
    }

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<Option<IssuedExpirationDateSignatures>, NymApiStorageError> {
        let Some(raw) = self
            .manager
            .get_master_expiration_date_signatures(expiration_date)
            .await?
        else {
            return Ok(None);
        };

        let signatures = deserialise_expiration_date_signatures(&raw.serialised_signatures)?;

        Ok(Some(IssuedExpirationDateSignatures {
            epoch_id: raw.epoch_id as u64,
            signatures,
        }))
    }

    async fn insert_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        sigs: &IssuedExpirationDateSignatures,
    ) -> Result<(), NymApiStorageError> {
        self.manager
            .insert_master_expiration_date_signatures(
                sigs.epoch_id as i64,
                expiration_date,
                &serialise_expiration_date_signatures(&sigs.signatures),
            )
            .await?;
        Ok(())
    }
}
