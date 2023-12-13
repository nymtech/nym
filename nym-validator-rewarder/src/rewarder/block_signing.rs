// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Decimal, Uint128};
use nym_validator_client::nyxd::Coin;
use nyxd_scraper::models;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ValidatorSigning {
    pub validator: models::Validator,

    pub voting_power_at_epoch_start: i64,
    pub voting_power_ratio: Decimal,

    pub signed_blocks: i32,
    pub ratio_signed: Decimal,
}

#[derive(Debug)]
pub struct EpochSigning {
    pub blocks: i64,
    pub total_voting_power_at_epoch_start: i64,

    pub validators: Vec<ValidatorSigning>,
}

impl EpochSigning {
    pub fn construct(
        blocks: i64,
        total_vp: i64,
        validator_results: HashMap<models::Validator, (i32, i64)>,
    ) -> Self {
        assert!(total_vp >= 0, "negative voting power!");
        assert!(blocks >= 0, "negative blocks!");
        let total_vp_u64: u64 = total_vp.try_into().unwrap_or_default();
        let blocks_u64: u64 = blocks.try_into().unwrap_or_default();

        let validators = validator_results
            .into_iter()
            .map(
                |(validator, (signed_blocks, voting_power_at_epoch_start))| {
                    let vp: u64 = voting_power_at_epoch_start.try_into().unwrap_or_default();
                    let signed: u64 = signed_blocks.try_into().unwrap_or_default();

                    let voting_power_ratio = Decimal::from_ratio(vp, total_vp_u64);
                    let ratio_signed = Decimal::from_ratio(signed, blocks_u64);

                    ValidatorSigning {
                        validator,
                        voting_power_at_epoch_start,
                        voting_power_ratio,
                        signed_blocks,
                        ratio_signed,
                    }
                },
            )
            .collect();

        EpochSigning {
            blocks,
            total_voting_power_at_epoch_start: total_vp,
            validators,
        }
    }

    pub fn rewarding_amounts(&self, budget: &Coin) -> HashMap<models::Validator, Coin> {
        let denom = &budget.denom;
        self.validators
            .iter()
            .map(|v| {
                let amount = Uint128::new(budget.amount) * v.ratio_signed * v.voting_power_ratio;

                (v.validator.clone(), Coin::new(amount.u128(), denom))
            })
            .collect()
    }
}
