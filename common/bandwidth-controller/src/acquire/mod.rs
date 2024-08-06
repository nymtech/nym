// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use crate::utils::{get_coin_index_signatures, get_expiration_date_signatures};
use log::info;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::IssuanceTicketBook;
use nym_credentials::ecash::utils::obtain_aggregate_wallet;
use nym_credentials::IssuedTicketBook;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::identity;
use nym_ecash_time::{ecash_default_expiration_date, Date};
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::EcashSigningClient;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, EcashQueryClient};
use nym_validator_client::nyxd::cosmwasm_client::ToSingletonContractData;
use nym_validator_client::EcashApiClient;
use rand::rngs::OsRng;

pub async fn make_deposit<C>(
    client: &C,
    client_id: &[u8],
    expiration: Option<Date>,
    ticketbook_type: TicketType,
) -> Result<IssuanceTicketBook, BandwidthControllerError>
where
    C: EcashSigningClient + EcashQueryClient + Sync,
{
    let mut rng = OsRng;
    let signing_key = identity::PrivateKey::new(&mut rng);
    let expiration = expiration.unwrap_or_else(ecash_default_expiration_date);

    let deposit_amount = client.get_required_deposit_amount().await?;
    info!("we'll need to deposit {deposit_amount} to obtain the ticketbook");
    let result = client
        .make_ticketbook_deposit(
            signing_key.public_key().to_base58_string(),
            deposit_amount.into(),
            None,
        )
        .await?;

    let deposit_id = result.parse_singleton_u32_contract_data()?;

    info!("our ticketbook deposit has been stored under id {deposit_id}");

    Ok(IssuanceTicketBook::new_with_expiration(
        deposit_id,
        client_id,
        signing_key,
        ticketbook_type,
        expiration,
    ))
}

pub async fn query_and_persist_required_global_signatures<S>(
    storage: &S,
    epoch_id: EpochId,
    expiration_date: Date,
    apis: Vec<EcashApiClient>,
) -> Result<(), BandwidthControllerError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    log::info!("Getting expiration date signatures");
    // this will also persist the signatures in the storage if they were not there already
    get_expiration_date_signatures(storage, epoch_id, expiration_date, apis.clone()).await?;

    log::info!("Getting coin indices signatures");
    // this will also persist the signatures in the storage if they were not there already
    get_coin_index_signatures(storage, epoch_id, apis).await?;
    Ok(())
}

pub async fn get_ticket_book<C, St>(
    issuance_data: &IssuanceTicketBook,
    client: &C,
    storage: &St,
    apis: Option<Vec<EcashApiClient>>,
) -> Result<IssuedTicketBook, BandwidthControllerError>
where
    C: DkgQueryClient + Send + Sync,
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let threshold = client
        .get_current_epoch_threshold()
        .await?
        .ok_or(BandwidthControllerError::NoThreshold)?;

    let apis = match apis {
        Some(apis) => apis,
        None => all_ecash_api_clients(client, epoch_id).await?,
    };

    log::info!("Querying wallet signatures");
    let wallet = obtain_aggregate_wallet(issuance_data, &apis, threshold).await?;
    info!("managed to obtain sufficient number of partial signatures!");

    log::info!("Getting expiration date signatures");
    // this will also persist the signatures in the storage if they were not there already
    get_expiration_date_signatures(
        storage,
        epoch_id,
        issuance_data.expiration_date(),
        apis.clone(),
    )
    .await?;

    log::info!("Getting coin indices signatures");
    // this will also persist the signatures in the storage if they were not there already
    get_coin_index_signatures(storage, epoch_id, apis).await?;

    let issued = issuance_data.to_issued_ticketbook(wallet, epoch_id);

    info!("persisting the ticketbook into the storage...");
    storage
        .insert_issued_ticketbook(&issued)
        .await
        .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?;
    Ok(issued)
}
