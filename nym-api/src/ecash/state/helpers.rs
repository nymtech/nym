// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::ecash::state::local::TicketDoubleSpendingFilter;
use crate::ecash::storage::EcashStorageExt;
use crate::support::storage::NymApiStorage;
use futures::{stream, StreamExt};
use nym_compact_ecash::constants;
use nym_config::defaults::BloomfilterParameters;
use nym_dkg::Threshold;
use nym_ecash_double_spending::{DoubleSpendingFilter, DoubleSpendingFilterBuilder};
use nym_ecash_time::{cred_exp_date, ecash_today};
use nym_validator_client::EcashApiClient;
use std::future::Future;
use time::ext::NumericalDuration;
use time::Date;
use tokio::sync::Mutex;

// attempt to completely rebuild the bloomfilter data for given day
async fn try_rebuild_today_bloomfilter(
    today: Date,
    params: BloomfilterParameters,
    storage: &NymApiStorage,
) -> Result<DoubleSpendingFilter, EcashError> {
    log::info!("rebuilding bloomfilter for {today}");

    let tickets = storage.get_all_spent_tickets_on(today).await?;
    log::debug!(
        "there are {} tickets to insert into the filter",
        tickets.len()
    );

    let mut filter = DoubleSpendingFilter::new_empty(params);
    for ticket in tickets {
        filter.set(&ticket.serial_number)
    }
    Ok(filter)
}

pub(crate) async fn prepare_partial_bloomfilter_builder(
    storage: &NymApiStorage,
    params: BloomfilterParameters,
    params_id: i64,
    start: Date,
    days: i64,
) -> Result<DoubleSpendingFilterBuilder, EcashError> {
    log::info!(
        "attempting to rebuild partial bloomfilter starting at {start} which includes {days} days"
    );

    let mut filter_builder = DoubleSpendingFilter::builder(params);
    for i in 0..days {
        let date = start - i.days();
        let Some(bitmap) = storage
            .try_load_partial_bloomfilter_bitmap(date, params_id)
            .await?
        else {
            log::warn!("missing double spending bloomfilter bitmap for {date} (if this API hasn't been running for at least {days} day(s) since 'ecash'-based zk-nyms were introduced this is expected)");
            continue;
        };
        if !filter_builder.add_bytes(&bitmap) {
            log::error!(
                "failed to add bitmap from {date} to the global bloomfilter. it may be malformed!"
            );
        }
    }
    Ok(filter_builder)
}

pub(super) async fn try_rebuild_bloomfilter(
    storage: &NymApiStorage,
) -> Result<TicketDoubleSpendingFilter, EcashError> {
    log::info!("attempting to rebuild the double spending bloomfilter...");
    let today = ecash_today().date();

    let (params_id, params) = storage.get_double_spending_filter_params().await?;
    log::info!("will use the following parameters: {params:?}");

    // we're never going to have persisted data for 'today'. we need to rebuild it from scratch
    let today_filter = try_rebuild_today_bloomfilter(today, params, storage).await?;

    log::info!("attempting to rebuild the global filter");
    let mut global_filter = prepare_partial_bloomfilter_builder(
        storage,
        params,
        params_id,
        today.previous_day().unwrap(),
        constants::CRED_VALIDITY_PERIOD_DAYS as i64 - 1,
    )
    .await?;

    if !global_filter.add_bytes(&today_filter.dump_bitmap()) {
        log::error!(
            "failed to add bitmap from {today} to the global bloomfilter. it may be malformed!"
        );
    }

    Ok(TicketDoubleSpendingFilter::new(
        today,
        params_id,
        global_filter.build(),
        today_filter,
    ))
}

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
                    log::warn!("failed to obtain partial threshold data from API: {disp}: {err}")
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
