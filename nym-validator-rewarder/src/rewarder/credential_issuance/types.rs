// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use crate::rewarder::epoch::Epoch;
use crate::rewarder::helpers::api_client;
use crate::rewarder::nyxd_client::NyxdClient;
use cosmwasm_std::{Addr, Decimal, Uint128};
use nym_compact_ecash::VerificationKeyAuth;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::{AccountId, Coin};
use std::collections::{HashMap, HashSet};
use std::mem;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Clone)]
pub struct MonitoringResults {
    inner: Arc<Mutex<MonitoringResultsInner>>,
}

impl MonitoringResults {
    pub(crate) async fn new_initial(
        initial_epoch: Epoch,
        nyxd_client: &NyxdClient,
        whitelist: &[AccountId],
    ) -> Result<Self, NymRewarderError> {
        let epoch = nyxd_client.dkg_epoch().await?;
        let issuers = nyxd_client.get_credential_issuers(epoch.epoch_id).await?;

        let mut initial_results = HashMap::new();

        for issuer in issuers {
            let issuer_account = issuer.operator_account.to_string();
            let mut raw_issuer = RawOperatorIssuing {
                api_runner: issuer.api_runner.clone(),
                runner_account: issuer.operator_account.clone(),
                whitelisted: whitelist.contains(&issuer.operator_account),
                per_epoch: Default::default(),
            };

            let Ok(api_client) = api_client(&issuer) else {
                warn!("failed to create api client for {issuer_account}");
                initial_results.insert(issuer_account, raw_issuer);
                continue;
            };

            let Ok(epoch_credentials) = api_client.epoch_credentials(epoch.epoch_id).await else {
                warn!("failed to get initial epoch credentials from {issuer_account}");
                initial_results.insert(issuer_account, raw_issuer);
                continue;
            };

            raw_issuer.per_epoch.insert(
                epoch.epoch_id as u32,
                IssuedEpochCredentials {
                    issued_since_monitor_started: 0,
                    validated_ids: Default::default(),
                    last_total_issued: epoch_credentials.total_issued,
                },
            );
            initial_results.insert(issuer_account, raw_issuer);
        }

        Ok(MonitoringResults {
            inner: Arc::new(Mutex::new(MonitoringResultsInner::new(
                initial_epoch,
                epoch.epoch_id as u32,
                initial_results,
            ))),
        })
    }

    pub(crate) async fn append_run_results(&self, dkg_epoch: u32, results: Vec<RawOperatorResult>) {
        let mut guard = self.inner.lock().await;

        for result in results {
            let Some(entry) = guard.operators.get_mut(result.operator_account.as_ref()) else {
                // if this is the first time we're seeing this data, make sure to set the current results as the starting point
                guard.operators.insert(
                    result.operator_account.to_string(),
                    RawOperatorIssuing::new_empty(dkg_epoch, result),
                );

                continue;
            };

            let Some(epoch_data) = entry.per_epoch.get_mut(&dkg_epoch) else {
                // similar situation to the above, if we don't have the proper initial data, set it to what we got now
                entry
                    .per_epoch
                    .insert(dkg_epoch, IssuedEpochCredentials::new_initial(&result));
                continue;
            };

            let issued = result.issued_credentials - epoch_data.last_total_issued;
            epoch_data.last_total_issued = result.issued_credentials;

            for validated in result.validated_credentials {
                epoch_data.validated_ids.insert(validated);
            }

            epoch_data.issued_since_monitor_started += issued;
        }
    }

    pub(crate) async fn finish_epoch(&self) -> MonitoringResultsInner {
        let mut guard = self.inner.lock().await;
        let next_epoch = guard.epoch.next();

        // safety: whenever the monitor results are constructed, we always have at least a single value there
        #[allow(clippy::unwrap_used)]
        let latest_dkg_epoch = guard.dkg_epochs.pop().unwrap();

        // only keep results from the latest dkg epoch (but after resetting the counters)
        let mut to_keep = HashMap::new();
        for (runner, result) in &guard.operators {
            let mut kept_epoch = HashMap::new();
            if let Some(data) = result.per_epoch.get(&latest_dkg_epoch) {
                kept_epoch.insert(
                    latest_dkg_epoch,
                    IssuedEpochCredentials {
                        issued_since_monitor_started: 0,
                        validated_ids: Default::default(),
                        last_total_issued: data.last_total_issued,
                    },
                );
            }

            to_keep.insert(
                runner.clone(),
                RawOperatorIssuing {
                    api_runner: result.api_runner.clone(),
                    runner_account: result.runner_account.clone(),
                    whitelisted: result.whitelisted,
                    per_epoch: kept_epoch,
                },
            );
        }

        let next_results = MonitoringResultsInner {
            epoch: next_epoch,
            dkg_epochs: vec![latest_dkg_epoch],
            operators: to_keep,
        };

        mem::replace(&mut guard, next_results)
    }
}

pub(crate) struct MonitoringResultsInner {
    pub(crate) epoch: Epoch,
    pub(crate) dkg_epochs: Vec<u32>,

    // map from operator's account to their results
    pub(crate) operators: HashMap<String, RawOperatorIssuing>,
}

impl From<MonitoringResultsInner> for CredentialIssuanceResults {
    fn from(value: MonitoringResultsInner) -> Self {
        let mut total_issued = 0;

        for operator in value.operators.values() {
            // if this validator is NOT whitelisted, do not increase the total issued credentials
            if operator.whitelisted {
                let operator_issued: u32 = operator
                    .per_epoch
                    .values()
                    .map(|e| e.issued_since_monitor_started)
                    .sum();

                total_issued += operator_issued
            }
        }

        CredentialIssuanceResults {
            total_issued_partial_credentials: total_issued,
            dkg_epochs: value.dkg_epochs,
            api_runners: value
                .operators
                .into_values()
                .map(|runner| {
                    let issued_ratio = if total_issued == 0 || !runner.whitelisted {
                        Decimal::zero()
                    } else {
                        Decimal::from_ratio(runner.issued_credentials(), total_issued)
                    };
                    OperatorIssuing {
                        issued_ratio,
                        issued_credentials: runner.issued_credentials(),
                        validated_credentials: runner.validated_credentials(),
                        api_runner: runner.api_runner,
                        whitelisted: runner.whitelisted,
                        runner_account: runner.runner_account,
                    }
                })
                .collect(),
        }
    }
}

impl MonitoringResultsInner {
    fn new(
        epoch: Epoch,
        initial_dkg_epoch: u32,
        initial_operators: HashMap<String, RawOperatorIssuing>,
    ) -> MonitoringResultsInner {
        MonitoringResultsInner {
            epoch,
            dkg_epochs: vec![initial_dkg_epoch],
            operators: initial_operators,
        }
    }
}

pub(crate) struct RawOperatorResult {
    pub(crate) operator_account: AccountId,
    pub(crate) api_runner: String,
    pub(crate) whitelisted: bool,

    // how many credentials the operator claims to have issued in **TOTAL** in this **DKG** epoch
    pub(crate) issued_credentials: u32,
    pub(crate) validated_credentials: Vec<i64>,
}

impl RawOperatorResult {
    pub(crate) fn new_empty(
        operator_account: AccountId,
        api_runner: String,
        whitelisted: bool,
    ) -> RawOperatorResult {
        RawOperatorResult {
            operator_account,
            api_runner,
            whitelisted,
            issued_credentials: 0,
            validated_credentials: Default::default(),
        }
    }
}

pub struct RawOperatorIssuing {
    pub api_runner: String,
    pub runner_account: AccountId,
    pub whitelisted: bool,

    pub per_epoch: HashMap<u32, IssuedEpochCredentials>,
}

impl RawOperatorIssuing {
    pub fn new_empty(epoch: u32, raw_result: RawOperatorResult) -> RawOperatorIssuing {
        let mut per_epoch = HashMap::new();
        per_epoch.insert(epoch, IssuedEpochCredentials::new_initial(&raw_result));
        RawOperatorIssuing {
            api_runner: raw_result.api_runner,
            runner_account: raw_result.operator_account,
            whitelisted: raw_result.whitelisted,
            per_epoch,
        }
    }

    pub fn issued_credentials(&self) -> u32 {
        self.per_epoch
            .values()
            .map(|e| e.issued_since_monitor_started)
            .sum()
    }

    pub fn validated_credentials(&self) -> u32 {
        let ids: usize = self.per_epoch.values().map(|e| e.validated_ids.len()).sum();
        ids as u32
    }
}

pub struct IssuedEpochCredentials {
    pub issued_since_monitor_started: u32,
    pub validated_ids: HashSet<i64>,
    pub last_total_issued: u32,
}

impl IssuedEpochCredentials {
    pub fn new_initial(raw: &RawOperatorResult) -> Self {
        IssuedEpochCredentials {
            issued_since_monitor_started: 0,
            validated_ids: raw.validated_credentials.iter().copied().collect(),
            last_total_issued: raw.issued_credentials,
        }
    }
}

pub struct OperatorIssuing {
    pub api_runner: String,
    pub whitelisted: bool,
    pub runner_account: AccountId,

    pub issued_ratio: Decimal,
    pub issued_credentials: u32,
    pub validated_credentials: u32,
}

impl OperatorIssuing {
    pub fn reward_amount(&self, issuance_budget: &Coin) -> Coin {
        if !self.whitelisted {
            return Coin::new(0, &issuance_budget.denom);
        }

        let amount = Uint128::new(issuance_budget.amount) * self.issued_ratio;

        Coin::new(amount.u128(), &issuance_budget.denom)
    }
}

pub struct CredentialIssuanceResults {
    pub total_issued_partial_credentials: u32,
    pub dkg_epochs: Vec<u32>,
    pub api_runners: Vec<OperatorIssuing>,
}

impl CredentialIssuanceResults {
    pub fn rewarding_amounts(&self, budget: &Coin) -> Vec<(AccountId, Vec<Coin>)> {
        self.api_runners
            .iter()
            .inspect(|a| {
                info!(
                    "operator {} will receive {} at address {} for credential issuance work (whitelisted: {})",
                    a.api_runner,
                    a.reward_amount(budget),
                    a.runner_account,
                    a.whitelisted
                );
            })
            .map(|v| (v.runner_account.clone(), vec![v.reward_amount(budget)]))
            .collect()
    }
}

#[derive(Debug)]
pub struct CredentialIssuer {
    pub public_key: ed25519::PublicKey,
    pub operator_account: AccountId,
    pub api_runner: String,
    pub verification_key: VerificationKeyAuth,
}

// safety: we're converting between different wrappers for bech32 addresses
// and we trust (reasonably so), the values coming out of registered dealers in the DKG contract
pub(crate) fn addr_to_account_id(addr: Addr) -> AccountId {
    #[allow(clippy::unwrap_used)]
    addr.as_str().parse().unwrap()
}
