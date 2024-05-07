// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use log::warn;
use nym_credential_storage::storage::Storage;
use nym_credentials_interface::{
    AnnotatedCoinIndexSignature, AnnotatedExpirationDateSignature, VerificationKeyAuth,
};
use nym_ecash_time::Date;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::EcashApiClient;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::fmt::Display;
use std::future::Future;

// it really doesn't need the RwLock because it's never moved across tasks,
// but we need all the Send/Sync action
#[derive(Default)]
pub(crate) struct ApiClientsWrapper(Option<Vec<EcashApiClient>>);

impl ApiClientsWrapper {
    pub(crate) async fn get_or_init<C>(
        &mut self,
        epoch_id: EpochId,
        dkg_client: &C,
    ) -> Result<Vec<EcashApiClient>, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
    {
        if let Some(cached) = &self.0 {
            return Ok(cached.clone());
        }

        let clients = all_ecash_api_clients(dkg_client, epoch_id).await?;

        // technically we don't have to be cloning all the clients here, but it's way simpler than
        // dealing with locking and whatnot given the performance penalty is negligible
        self.0 = Some(clients.clone());
        Ok(clients)
    }
}

pub(crate) async fn query_random_apis_until_success<F, T, U, E>(
    mut apis: Vec<EcashApiClient>,
    f: F,
    typ: impl Into<String>,
) -> Result<T, BandwidthControllerError>
where
    F: Fn(EcashApiClient) -> U,
    U: Future<Output = Result<T, E>>,
    E: Display,
{
    // try apis in pseudorandom way to remove any bias towards the first registered dealer
    apis.shuffle(&mut thread_rng());

    for api in apis {
        let disp = api.to_string();
        match f(api).await {
            Ok(res) => return Ok(res),
            Err(err) => {
                warn!("failed to obtain valid response from API {disp}: {err}")
            }
        }
    }
    Err(BandwidthControllerError::ExhaustedApiQueries { typ: typ.into() })
}

pub(crate) async fn get_aggregate_verification_key<St>(
    storage: &St,
    epoch_id: EpochId,
    ecash_apis: Vec<EcashApiClient>,
) -> Result<VerificationKeyAuth, BandwidthControllerError>
where
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    if let Some(stored) = storage
        .get_master_verification_key(epoch_id)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?
    {
        return Ok(stored);
    };

    let master_vk = query_random_apis_until_success(
        ecash_apis,
        |api| async move { api.api_client.master_verification_key(Some(epoch_id)).await },
        format!("aggregated verification key for epoch {epoch_id}"),
    )
    .await?
    .key;

    // store the retrieved key
    storage
        .insert_master_verification_key(epoch_id, &master_vk)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?;

    Ok(master_vk)
}

pub(crate) async fn get_coin_index_signatures<St>(
    storage: &St,
    epoch_id: EpochId,
    ecash_apis: Vec<EcashApiClient>,
) -> Result<Vec<AnnotatedCoinIndexSignature>, BandwidthControllerError>
where
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    if let Some(stored) = storage
        .get_coin_index_signatures(epoch_id)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?
    {
        return Ok(stored);
    };

    let index_sigs = query_random_apis_until_success(
        ecash_apis,
        |api| async move {
            api.api_client
                .global_coin_indices_signatures(Some(epoch_id))
                .await
        },
        format!("aggregated coin index signatures for epoch {epoch_id}"),
    )
    .await?
    .signatures;

    // store the retrieved key
    storage
        .insert_coin_index_signatures(epoch_id, &index_sigs)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?;

    Ok(index_sigs)
}

pub(crate) async fn get_expiration_date_signatures<St>(
    storage: &St,
    epoch_id: EpochId,
    expiration_date: Date,
    ecash_apis: Vec<EcashApiClient>,
) -> Result<Vec<AnnotatedExpirationDateSignature>, BandwidthControllerError>
where
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    if let Some(stored) = storage
        .get_expiration_date_signatures(expiration_date)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?
    {
        return Ok(stored);
    };

    let expiration_sigs = query_random_apis_until_success(
        ecash_apis,
        |api| async move {
            api.api_client
                .global_expiration_date_signatures(Some(expiration_date))
                .await
        },
        format!("aggregated coin index signatures for date {expiration_date}"),
    )
    .await?
    .signatures;

    // store the retrieved key
    storage
        .insert_expiration_date_signatures(epoch_id, expiration_date, &expiration_sigs)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?;

    Ok(expiration_sigs)
}
