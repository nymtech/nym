// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, StreamExt};
use nym_validator_client::EcashApiClient;
use std::future::Future;
use std::ops::Deref;
use tokio::sync::RwLockReadGuard;

pub(crate) fn apis_stream<'a>(
    // if needed we could make this argument more generic to accept either locks or iterators, etc.
    all_clients: &'a RwLockReadGuard<'a, Vec<EcashApiClient>>,
    filter_by_id: &'a [u64],
) -> impl Stream<Item = &'a EcashApiClient> + 'a {
    // this vector will never contain more than ~30 entries so linear lookup is fine.
    // it's probably even faster than hashset due to overhead
    futures::stream::iter(
        all_clients
            .deref()
            .iter()
            .filter(|client| filter_by_id.contains(&client.node_id)),
    )
}

pub(crate) async fn for_each_api_concurrent<'a, F, Fut>(
    all_clients: &'a RwLockReadGuard<'a, Vec<EcashApiClient>>,
    filter_by_id: &'a [u64],
    f: F,
) where
    F: FnMut(&'a EcashApiClient) -> Fut,
    Fut: Future<Output = ()>,
{
    apis_stream(all_clients, filter_by_id)
        .for_each_concurrent(32, f)
        .await
}
