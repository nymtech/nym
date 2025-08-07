// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::http::state::ChainClient;
use nym_validator_client::nyxd::contract_traits::EcashQueryClient;
use nym_validator_client::nyxd::Coin;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;

pub(crate) struct CachedDeposit {
    valid_until: OffsetDateTime,
    required_amount: Coin,
}

impl CachedDeposit {
    const MAX_VALIDITY: time::Duration = time::Duration::MINUTE;

    fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    fn update(&mut self, required_amount: Coin) {
        self.valid_until = OffsetDateTime::now_utc() + Self::MAX_VALIDITY;
        self.required_amount = required_amount;
    }
}

impl Default for CachedDeposit {
    fn default() -> Self {
        CachedDeposit {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            required_amount: Coin {
                amount: u128::MAX,
                denom: "unym".to_string(),
            },
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct RequiredDepositCache {
    inner: Arc<RwLock<CachedDeposit>>,
}

impl RequiredDepositCache {
    pub(crate) async fn get_or_update(
        &self,
        chain_client: &ChainClient,
    ) -> Result<Coin, CredentialProxyError> {
        let read_guard = self.inner.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.required_amount.clone());
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.inner.write().await;
        let deposit_amount = chain_client
            .query_chain()
            .await
            .get_required_deposit_amount()
            .await?;

        let nym_coin: Coin = deposit_amount.into();

        write_guard.update(nym_coin.clone());
        Ok(nym_coin)
    }
}
