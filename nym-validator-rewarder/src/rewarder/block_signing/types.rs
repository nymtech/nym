// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::helpers::{consensus_pubkey_to_address, operator_account_to_owner_account};
use cosmwasm_std::{Decimal, Uint128};
use nym_validator_client::nyxd::module_traits::staking;
use nym_validator_client::nyxd::{AccountId, Coin};
use nyxd_scraper::models;
use std::collections::HashMap;
use tracing::info;

#[derive(Debug)]
pub struct ValidatorSigning {
    pub validator: models::Validator,
    pub staking_details: staking::Validator,
    pub operator_account: AccountId,
    pub whitelisted: bool,

    pub voting_power_at_epoch_start: i64,
    pub voting_power_ratio: Decimal,

    pub signed_blocks: i32,
    pub ratio_signed: Decimal,
}

impl ValidatorSigning {
    pub fn moniker(&self) -> String {
        self.staking_details
            .description
            .as_ref()
            .map(|d| d.moniker.clone())
            .unwrap_or("UNKNOWN MONIKER".to_string())
    }

    pub fn reward_amount(&self, signing_budget: &Coin) -> Coin {
        if !self.whitelisted {
            return Coin::new(0, &signing_budget.denom);
        }

        let amount =
            Uint128::new(signing_budget.amount) * self.ratio_signed * self.voting_power_ratio;

        Coin::new(amount.u128(), &signing_budget.denom)
    }
}

#[derive(Debug)]
pub struct EpochSigningResults {
    pub blocks: i64,
    pub total_voting_power_at_epoch_start: i64,

    pub validators: Vec<ValidatorSigning>,
}

#[derive(Debug)]
pub struct RawValidatorResult {
    pub signed_blocks: i32,
    pub voting_power: i64,
    pub whitelisted: bool,
}

impl RawValidatorResult {
    pub fn new(signed_blocks: i32, voting_power: i64, whitelisted: bool) -> Self {
        Self {
            signed_blocks,
            voting_power,
            whitelisted,
        }
    }
}

impl EpochSigningResults {
    pub fn construct(
        blocks: i64,
        total_vp: i64,
        validator_results: HashMap<models::Validator, RawValidatorResult>,
        validator_details: Vec<staking::Validator>,
    ) -> Result<Self, NymRewarderError> {
        let Ok(total_vp_u64): Result<u64, _> = total_vp.try_into() else {
            return Err(NymRewarderError::NegativeTotalVotingPower { val: total_vp });
        };
        let Ok(blocks_u64): Result<u64, _> = blocks.try_into() else {
            return Err(NymRewarderError::NegativeSignedBlocks { val: blocks });
        };

        let mut validator_details: HashMap<_, _> = validator_details
            .into_iter()
            .filter(|v| v.consensus_pubkey.is_some())
            .map(|v| {
                // safety: we know the key is definitely set as we just filtered the iterator based on that condition
                #[allow(clippy::unwrap_used)]
                consensus_pubkey_to_address(v.consensus_pubkey.unwrap())
                    .map(|addr| (addr.to_string(), v))
            })
            .collect::<Result<_, _>>()?;

        let mut validators = Vec::new();

        for (validator, raw_results) in validator_results {
            let vp: u64 = raw_results.voting_power.try_into().unwrap_or_default();
            let signed: u64 = raw_results.signed_blocks.try_into().unwrap_or_default();

            let voting_power_ratio = if raw_results.whitelisted {
                Decimal::from_ratio(vp, total_vp_u64)
            } else {
                Decimal::zero()
            };

            debug_assert!(signed <= blocks_u64);
            let ratio_signed = Decimal::from_ratio(signed, blocks_u64);
            let staking_details = validator_details
                .remove(&validator.consensus_address)
                .ok_or_else(|| NymRewarderError::MissingValidatorDetails {
                    consensus_address: validator.consensus_address.clone(),
                })?;

            let operator_account =
                operator_account_to_owner_account(&staking_details.operator_address)?;

            validators.push(ValidatorSigning {
                validator,
                staking_details,
                operator_account,
                whitelisted: raw_results.whitelisted,
                voting_power_at_epoch_start: raw_results.voting_power,
                voting_power_ratio,
                signed_blocks: raw_results.signed_blocks,
                ratio_signed,
            })
        }

        Ok(EpochSigningResults {
            blocks,
            total_voting_power_at_epoch_start: total_vp,
            validators,
        })
    }

    pub fn rewarding_amounts(&self, budget: &Coin) -> Vec<(AccountId, Vec<Coin>)> {
        let mut amounts = Vec::with_capacity(self.validators.len());

        for v in &self.validators {
            let amount = v.reward_amount(budget);
            info!(
                    "validator {} will receive {amount} at address {} for block signing work (whitelisted: {})",
                    v.moniker(),
                    v.operator_account,
                    v.whitelisted
                );
            amounts.push((v.operator_account.clone(), vec![amount]))
        }

        amounts
    }
}
