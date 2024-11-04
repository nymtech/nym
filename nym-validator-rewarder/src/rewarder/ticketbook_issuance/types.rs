// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::rewarder::ticketbook_issuance::verifier::IssuerBan;
use cosmwasm_std::{Addr, Decimal, Uint128};
use nym_coconut_dkg_common::types::NodeIndex;
use nym_compact_ecash::VerificationKeyAuth;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::{AccountId, Coin};
use nym_validator_client::NymApiClient;
use std::fmt::{Display, Formatter};
use tracing::info;

pub struct OperatorIssuing {
    pub api_runner: String,
    pub whitelisted: bool,
    pub pre_banned: bool,
    pub runner_account: AccountId,

    // TODO: split reward into 1/whitelist size
    // then issued ratio is ratio of deposits in that time interval and thus your slice of the 1/whitelist
    pub issued_ratio: Decimal,
    pub skipped_verification: bool,
    pub subsample_size: u32,
    pub issued_ticketbooks: u32,
    pub issuer_ban: Option<IssuerBan>,
}

impl OperatorIssuing {
    pub fn reward_amount(&self, operator_budget: &Coin) -> Coin {
        if !self.whitelisted || self.issuer_ban.is_some() {
            return Coin::new(0, &operator_budget.denom);
        }

        let amount = Uint128::new(operator_budget.amount) * self.issued_ratio;

        Coin::new(amount.u128(), &operator_budget.denom)
    }
}

pub struct TicketbookIssuanceResults {
    pub approximate_deposits: u32,
    pub api_runners: Vec<OperatorIssuing>,
}

impl TicketbookIssuanceResults {
    pub fn rewarding_amounts(&self, per_operator_budget: &Coin) -> Vec<(AccountId, Vec<Coin>)> {
        info!("our budget per operator is: {per_operator_budget}");

        let mut amounts = Vec::with_capacity(self.api_runners.len());
        for api_runner in &self.api_runners {
            let amount = api_runner.reward_amount(per_operator_budget);
            info!(
                    "operator {} will receive {amount} at address {} for credential issuance work (whitelisted: {})",
                    api_runner.api_runner,
                    api_runner.runner_account,
                    api_runner.whitelisted
                );
            amounts.push((api_runner.runner_account.clone(), vec![amount]))
        }

        amounts
    }
}

#[derive(Clone)]
pub struct CredentialIssuer {
    pub public_key: ed25519::PublicKey,
    pub operator_account: AccountId,
    pub api_client: NymApiClient,
    pub verification_key: VerificationKeyAuth,
    pub node_id: NodeIndex,
}

impl Display for CredentialIssuer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[id: {}] {} @ {}",
            self.node_id,
            self.operator_account,
            self.api_client.api_url()
        )
    }
}

// safety: we're converting between different wrappers for bech32 addresses
// and we trust (reasonably so), the values coming out of registered dealers in the DKG contract
pub(crate) fn addr_to_account_id(addr: Addr) -> AccountId {
    #[allow(clippy::unwrap_used)]
    addr.as_str().parse().unwrap()
}
