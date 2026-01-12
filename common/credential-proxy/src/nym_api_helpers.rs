// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: this was just copied from nym-api;
// it should have been therefore extracted to a common crate instead and imported as dependency

use crate::error::CredentialProxyError;
use futures::{StreamExt, stream};
use nym_cache::CachedImmutableItems;
use nym_credentials::ecash::utils::{EcashTime, cred_exp_date, ecash_today};
use nym_validator_client::EcashApiClient;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use std::cmp::min;
use std::future::Future;
use time::{Date, OffsetDateTime};
use tokio::sync::Mutex;
use tracing::warn;

pub struct CachedEpoch {
    valid_until: OffsetDateTime,
    pub current_epoch: Epoch,
}

impl Default for CachedEpoch {
    fn default() -> Self {
        CachedEpoch {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            current_epoch: Epoch::default(),
        }
    }
}

impl CachedEpoch {
    pub fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    pub fn update(&mut self, epoch: Epoch) {
        let now = OffsetDateTime::now_utc();

        let validity_duration = if let Some(epoch_finish) = epoch.deadline {
            #[allow(clippy::unwrap_used)]
            let state_end =
                OffsetDateTime::from_unix_timestamp(epoch_finish.seconds() as i64).unwrap();
            let until_epoch_state_end = state_end - now;
            // make it valid until the next epoch transition or next 5min, whichever is smaller
            min(until_epoch_state_end, 5 * time::Duration::MINUTE)
        } else {
            5 * time::Duration::MINUTE
        };

        self.valid_until = now + validity_duration;
        self.current_epoch = epoch;
    }
}

// an item that stays constant throughout given epoch
pub type CachedImmutableEpochItem<T> = CachedImmutableItems<EpochId, T>;

pub fn ensure_sane_expiration_date(expiration_date: Date) -> Result<(), CredentialProxyError> {
    let today = ecash_today();

    if expiration_date < today.date() {
        // what's the point of signatures with expiration in the past?
        return Err(CredentialProxyError::ExpirationDateTooEarly);
    }

    if expiration_date > cred_exp_date().ecash_date() {
        return Err(CredentialProxyError::ExpirationDateTooLate);
    }

    Ok(())
}

pub async fn query_all_threshold_apis<F, T, U>(
    all_apis: Vec<EcashApiClient>,
    threshold: u64,
    f: F,
) -> Result<Vec<T>, CredentialProxyError>
where
    F: Fn(EcashApiClient) -> U,
    U: Future<Output = Result<T, CredentialProxyError>>,
{
    let shares = Mutex::new(Vec::with_capacity(all_apis.len()));

    stream::iter(all_apis)
        .for_each_concurrent(8, |api| async {
            // can't be bothered to restructure the code to appease the borrow checker properly,
            // so just assign this to a variable
            let disp = api.to_string();
            match f(api).await {
                Ok(partial_share) => shares.lock().await.push(partial_share),
                Err(err) => {
                    warn!("failed to obtain partial threshold data from API: {disp}: {err}")
                }
            }
        })
        .await;

    let shares = shares.into_inner();

    if shares.len() < threshold as usize {
        return Err(CredentialProxyError::InsufficientNumberOfSigners {
            threshold,
            available: shares.len(),
        });
    }

    Ok(shares)
}
