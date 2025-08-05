// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use log::warn;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
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

pub(crate) trait EcashClientsProvider {
    async fn try_get_ecash_clients(
        &mut self,
    ) -> Result<Vec<EcashApiClient>, BandwidthControllerError>;
}

impl EcashClientsProvider for Vec<EcashApiClient> {
    async fn try_get_ecash_clients(
        &mut self,
    ) -> Result<Vec<EcashApiClient>, BandwidthControllerError> {
        Ok(self.clone())
    }
}

impl<C> EcashClientsProvider for &mut ApiClientsWrapper<'_, C>
where
    C: DkgQueryClient + Sync + Send,
{
    async fn try_get_ecash_clients(
        &mut self,
    ) -> Result<Vec<EcashApiClient>, BandwidthControllerError> {
        self.clients().await
    }
}

pub(crate) enum ApiClientsWrapper<'a, C> {
    Uninitialised {
        query_client: &'a C,
        epoch_id: EpochId,
    },
    Cached {
        clients: Vec<EcashApiClient>,
    },
}

impl<'a, C> ApiClientsWrapper<'a, C> {
    pub(crate) fn new(query_client: &'a C, epoch_id: EpochId) -> Self {
        ApiClientsWrapper::Uninitialised {
            query_client,
            epoch_id,
        }
    }

    async fn clients(&mut self) -> Result<Vec<EcashApiClient>, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
    {
        match self {
            ApiClientsWrapper::Uninitialised {
                query_client,
                epoch_id,
            } => {
                let clients = all_ecash_api_clients(*query_client, *epoch_id).await?;
                *self = ApiClientsWrapper::Cached {
                    clients: clients.clone(),
                };

                Ok(clients)
            }
            ApiClientsWrapper::Cached { clients } => Ok(clients.clone()),
        }
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
    mut ecash_apis: impl EcashClientsProvider,
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

    let ecash_apis = ecash_apis.try_get_ecash_clients().await?;

    let master_vk = query_random_apis_until_success(
        ecash_apis,
        |api| async move { api.api_client.master_verification_key(Some(epoch_id)).await },
        format!("aggregated verification key for epoch {epoch_id}"),
    )
    .await?
    .key;

    let full = EpochVerificationKey {
        epoch_id,
        key: master_vk,
    };

    // store the retrieved key
    storage
        .insert_master_verification_key(&full)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?;

    Ok(full.key)
}

pub(crate) async fn get_coin_index_signatures<St>(
    storage: &St,
    epoch_id: EpochId,
    mut ecash_apis: impl EcashClientsProvider,
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

    let ecash_apis = ecash_apis.try_get_ecash_clients().await?;

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

    let aggregated = AggregatedCoinIndicesSignatures {
        epoch_id,
        signatures: index_sigs,
    };

    // store the retrieved key
    storage
        .insert_coin_index_signatures(&aggregated)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?;

    Ok(aggregated.signatures)
}

pub(crate) async fn get_expiration_date_signatures<St>(
    storage: &St,
    epoch_id: EpochId,
    expiration_date: Date,
    mut ecash_apis: impl EcashClientsProvider,
) -> Result<Vec<AnnotatedExpirationDateSignature>, BandwidthControllerError>
where
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    if let Some(stored) = storage
        .get_expiration_date_signatures(expiration_date, epoch_id)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?
    {
        return Ok(stored);
    };

    let ecash_apis = ecash_apis.try_get_ecash_clients().await?;

    let expiration_sigs = query_random_apis_until_success(
        ecash_apis,
        |api| async move {
            api.api_client
                .global_expiration_date_signatures(Some(expiration_date), Some(epoch_id))
                .await
        },
        format!("aggregated coin index signatures for date {expiration_date}"),
    )
    .await?
    .signatures;

    let aggregated = AggregatedExpirationDateSignatures {
        epoch_id,
        expiration_date,
        signatures: expiration_sigs,
    };

    // store the retrieved key
    storage
        .insert_expiration_date_signatures(&aggregated)
        .await
        .map_err(BandwidthControllerError::credential_storage_error)?;

    Ok(aggregated.signatures)
}
