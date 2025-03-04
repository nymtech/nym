// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use futures::{stream, StreamExt};
use nym_dkg::Threshold;
use nym_ecash_time::{cred_exp_date, ecash_today};
use nym_validator_client::EcashApiClient;
use std::future::Future;
use time::Date;
use tokio::sync::Mutex;
use tracing::warn;

pub(crate) fn ensure_sane_expiration_date(expiration_date: Date) -> Result<(), EcashError> {
    let today = ecash_today();

    if expiration_date < today.date() {
        // what's the point of signatures with expiration in the past?
        return Err(EcashError::ExpirationDateTooEarly);
    }

    // SAFETY: we're nowhere near MAX date
    #[allow(clippy::unwrap_used)]
    if expiration_date > cred_exp_date().date().next_day().unwrap() {
        // don't allow issuing signatures too far in advance (1 day beyond current value is fine)
        return Err(EcashError::ExpirationDateTooLate);
    }

    Ok(())
}

pub(crate) async fn query_all_threshold_apis<F, T, U>(
    all_apis: Vec<EcashApiClient>,
    threshold: Threshold,
    f: F,
) -> Result<Vec<T>, EcashError>
where
    F: Fn(EcashApiClient) -> U,
    U: Future<Output = Result<T, EcashError>>,
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
        return Err(EcashError::InsufficientNumberOfShares {
            threshold,
            shares: shares.len(),
        });
    }

    Ok(shares)
}
