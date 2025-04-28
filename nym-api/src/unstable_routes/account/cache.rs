use std::{sync::Arc, time::Duration};

use moka::{future::Cache, Entry};
use nym_validator_client::nyxd::AccountId;
use tokio::sync::RwLock;

use crate::{
    node_status_api::models::AxumResult,
    support::http::state::AppState,
    unstable_routes::{
        account::data_collector::AddressDataCollector,
        models::{NyxAccountDelegationDetails, NyxAccountDetails},
    },
};

#[derive(Clone)]
pub(crate) struct AddressInfoCache {
    inner: Cache<String, Arc<RwLock<NyxAccountDetails>>>,
}

impl AddressInfoCache {
    pub(crate) fn new() -> Self {
        // epoch duration = 1 hour
        // cache TTL is slightly lower than that to avoid too stale data in case
        // cache was refreshed JUST BEFORE epoch transition
        let cache_ttl = Duration::from_secs(60 * 30);
        let max_capacity = 1000;

        AddressInfoCache {
            inner: Cache::builder()
                .time_to_live(cache_ttl)
                .max_capacity(max_capacity)
                .build(),
        }
    }

    async fn upsert_address_info(
        &self,
        address: &str,
        address_info: NyxAccountDetails,
    ) -> Entry<String, Arc<RwLock<NyxAccountDetails>>> {
        self.inner
            .entry_by_ref(address)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = address_info;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(address_info))
                }
            })
            .await
    }

    pub(crate) async fn get_address_info(
        &self,
        state: AppState,
        account_id: AccountId,
    ) -> AxumResult<NyxAccountDetails> {
        let address = account_id.to_string();
        match self.inner.get(&address).await {
            Some(guard) => {
                tracing::trace!("Fetching from cache...");
                let read_lock = guard.read().await;
                Ok(read_lock.clone())
            }
            None => {
                tracing::trace!("No cache for {}, refreshing data...", &address);

                let address_info = self.collect_balances(state, &address, account_id).await?;

                self.upsert_address_info(&address, address_info.clone())
                    .await;

                Ok(address_info)
            }
        }
    }

    async fn collect_balances(
        &self,
        state: AppState,
        address: &str,
        account_id: AccountId,
    ) -> AxumResult<NyxAccountDetails> {
        let state = Arc::new(state);
        let mut collector = AddressDataCollector::new(state, account_id.clone());

        // ==> get balances of chain tokens <==
        let balance = collector.get_address_balance().await?;

        // it's very difficult to lower existing balance to exactly 0
        // so assume this is an unused address and return early
        if balance.amount == 0 {
            let address_info = NyxAccountDetails {
                address: address.to_string(),
                balance: balance.clone().into(),
                total_value: balance.clone().into(),
                delegations: Vec::new(),
                accumulated_rewards: Vec::new(),
                total_delegations: balance.clone().into(),
                claimable_rewards: balance.clone().into(),
                operator_rewards: None,
            };

            return Ok(address_info);
        }

        // ==> get list of delegations (history) <==
        let delegation_data = collector.get_delegations().await?;

        // ==> get the current reward for each active delegation <==
        // calculate rewards from nodes this delegator delegated to
        let accumulated_rewards = collector.calculate_rewards(&delegation_data).await?;

        // ==> convert totals <==
        let claimable_rewards = collector.claimable_rewards();
        let total_value = collector.total_value();
        let total_delegations = collector.total_delegations();
        let operator_rewards = collector.operator_rewards();

        let address_info = NyxAccountDetails {
            address: account_id.to_string(),
            balance: balance.into(),
            delegations: delegation_data
                .delegations()
                .into_iter()
                .map(|d| NyxAccountDelegationDetails {
                    delegated: d.amount,
                    height: d.height,
                    node_id: d.node_id,
                    proxy: d.proxy,
                })
                .collect(),
            accumulated_rewards,
            total_delegations,
            claimable_rewards,
            total_value,
            operator_rewards,
        };

        Ok(address_info)
    }
}
