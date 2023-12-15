// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use crate::rewarder::epoch::Epoch;
use cosmwasm_std::{Addr, Decimal, Uint128};
use nym_coconut::VerificationKey;
use nym_coconut_dkg_common::verification_key::ContractVKShare;
use nym_validator_client::nyxd::{AccountId, Coin};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct MonitoringResults {
    inner: Arc<Mutex<MonitoringResultsInner>>,
}

impl MonitoringResults {
    pub(crate) fn new(initial_epoch: Epoch) -> Self {
        MonitoringResults {
            inner: Arc::new(Mutex::new(MonitoringResultsInner::new(initial_epoch))),
        }
    }

    pub(crate) async fn api_endpoints(&self) -> Vec<String> {
        self.inner
            .lock()
            .await
            .operators
            .iter()
            .map(|o| o.api_runner.clone())
            .collect()
    }

    pub(crate) async fn set_epoch_operators(
        &self,
        dkg_epoch: u32,
        operators: Vec<(String, AccountId)>,
    ) {
        todo!()
        // let mut guard = self.inner.lock().await;
        // guard.operators = operators
        //     .into_iter()
        //     .map(|(api_runner, runner_account)| RawOperatorIssuing {
        //         api_runner,
        //         runner_account,
        //         issued_credentials: 0,
        //         validated_credentials: 0,
        //     })
        //     .collect();
        // guard.dkg_epoch = Some(dkg_epoch)
    }

    pub(crate) async fn append_run_results(&self, results: Vec<(String, RawOperatorResult)>) {
        todo!()
        // let mut guard = self.inner.lock().await;
        //
        // // sure, a hashmap would have been quicker, but we'll have at most 30-40 runners so
        // // performance overhead is negligible
        // for (api_runner, results) in results {
        //     if let Some(entry) = guard
        //         .operators
        //         .iter_mut()
        //         .find(|o| o.api_runner == api_runner)
        //     {
        //         entry.issued_credentials += results.issued_credentials;
        //         entry.validated_credentials += results.validated_credentials;
        //     } else {
        //         error!("somehow could not find operator results for runner {api_runner}!")
        //     }
        // }
    }

    pub(crate) async fn finish_epoch(&self) -> MonitoringResultsInner {
        todo!()
        // let mut guard = self.inner.lock().await;
        // let next_epoch = guard.epoch.next();
        // let next_results = MonitoringResultsInner::new(next_epoch);
        // mem::replace(&mut guard, next_results)
    }
}

pub(crate) struct MonitoringResultsInner {
    pub(crate) epoch: Epoch,
    pub(crate) dkg_epoch: Option<u32>,
    pub(crate) operators: Vec<RawOperatorIssuing>,
}

impl From<MonitoringResultsInner> for CredentialIssuanceResults {
    fn from(value: MonitoringResultsInner) -> Self {
        // approximation!
        let total_issued = value
            .operators
            .iter()
            .map(|o| o.issued_credentials)
            .max()
            .unwrap_or_default();

        CredentialIssuanceResults {
            total_issued,
            dkg_epoch: value.dkg_epoch,
            api_runners: value
                .operators
                .into_iter()
                .map(|runner| {
                    let issued_ratio = if total_issued == 0 {
                        Decimal::zero()
                    } else {
                        Decimal::from_ratio(runner.issued_credentials, total_issued)
                    };
                    OperatorIssuing {
                        api_runner: runner.api_runner,
                        runner_account: runner.runner_account,
                        issued_ratio,
                        issued_credentials: runner.issued_credentials,
                        validated_credentials: runner.validated_credentials,
                    }
                })
                .collect(),
        }
    }
}

impl MonitoringResultsInner {
    fn new(epoch: Epoch) -> MonitoringResultsInner {
        MonitoringResultsInner {
            epoch,
            dkg_epoch: None,
            operators: vec![],
        }
    }
}

pub(crate) struct RawOperatorResult {
    pub(crate) issued_credentials: u32,
    pub(crate) validated_credentials: u32,
}

pub struct RawOperatorIssuing {
    pub api_runner: String,
    pub runner_account: AccountId,

    pub issued_credentials: u32,
    pub validated_credentials: u32,

    pub(crate) starting_credential_id: u32,
}

pub struct OperatorIssuing {
    pub api_runner: String,
    pub runner_account: AccountId,

    pub issued_ratio: Decimal,
    pub issued_credentials: u32,
    pub validated_credentials: u32,
}

impl OperatorIssuing {
    pub fn reward_amount(&self, signing_budget: &Coin) -> Coin {
        let amount = Uint128::new(signing_budget.amount) * self.issued_ratio;

        Coin::new(amount.u128(), &signing_budget.denom)
    }
}

pub struct CredentialIssuanceResults {
    // note: this is an approximation!
    pub total_issued: u32,
    pub dkg_epoch: Option<u32>,
    pub api_runners: Vec<OperatorIssuing>,
}

impl CredentialIssuanceResults {
    pub fn rewarding_amounts(&self, budget: &Coin) -> Vec<(AccountId, Vec<Coin>)> {
        self.api_runners
            .iter()
            .inspect(|a| {
                info!(
                    "operator {} will receive {} at address {} for credential issuance work",
                    a.api_runner,
                    a.reward_amount(budget),
                    a.runner_account,
                );
            })
            .map(|v| (v.runner_account.clone(), vec![v.reward_amount(budget)]))
            .collect()
    }
}

pub struct CredentialIssuer {
    pub operator_account: AccountId,
    pub api_runner: String,
    pub verification_key: VerificationKey,
}

impl TryFrom<ContractVKShare> for CredentialIssuer {
    type Error = NymRewarderError;

    fn try_from(value: ContractVKShare) -> Result<Self, Self::Error> {
        todo!()
    }
}

// safety: we're converting between different wrappers for bech32 addresses
// and we trust (reasonably so), the values coming out of registered dealers in the DKG contract
fn addr_to_account_id(addr: Addr) -> AccountId {
    #[allow(clippy::unwrap_used)]
    addr.as_str().parse().unwrap()
}
