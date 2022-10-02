// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use validator_client::nymd::ValidatorResponse;

use crate::cache::Cache;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct PrettyValidatorInfo {
    pub address: String,
    pub pub_key: String,
    pub voting_power: u64,
    pub name: Option<String>,
}

pub(crate) struct ValidatorCache {
    pub(crate) validators: Cache<String, PrettyValidatorInfo>,
    pub(crate) summary: ValidatorSummary,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct ValidatorSummary {
    pub(crate) count: i32,
    pub(crate) block_height: u64,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeValidatorCache {
    inner: Arc<RwLock<ValidatorCache>>,
}

impl ThreadsafeValidatorCache {
    pub(crate) fn new() -> Self {
        ThreadsafeValidatorCache {
            inner: Arc::new(RwLock::new(ValidatorCache {
                validators: Cache::new(),
                summary: ValidatorSummary {
                    block_height: 0,
                    count: 0,
                },
            })),
        }
    }

    pub(crate) async fn get_validators(&self) -> Vec<PrettyValidatorInfo> {
        self.inner.read().await.validators.get_all()
    }

    pub(crate) async fn get_validator_summary(&self) -> ValidatorSummary {
        self.inner.read().await.summary.clone()
    }

    pub(crate) async fn update_cache(&self, validator_response: ValidatorResponse) {
        let mut guard = self.inner.write().await;

        for validator in validator_response.validators {
            let address = validator.address.to_string();
            guard.validators.set(
                address.clone(),
                PrettyValidatorInfo {
                    address,
                    pub_key: validator.pub_key.to_hex(),
                    name: validator.name,
                    voting_power: validator.power.value(),
                },
            )
        }

        guard.summary = ValidatorSummary {
            count: validator_response.total,
            block_height: validator_response.block_height.value(),
        };
    }
}
