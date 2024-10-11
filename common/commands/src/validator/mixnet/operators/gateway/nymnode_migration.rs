// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use cosmwasm_std::Uint128;
use log::info;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{
    NodeCostParams, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, DEFAULT_PROFIT_MARGIN_PERCENT,
};
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;
use nym_validator_client::nyxd::CosmWasmCoin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub profit_margin_percent: Option<u64>,

    #[clap(
        long,
        help = "operating cost in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub interval_operating_cost: Option<u128>,
}

pub async fn migrate_to_nymnode(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    let cost_params =
        if args.profit_margin_percent.is_some() || args.interval_operating_cost.is_some() {
            Some(NodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(
                    args.profit_margin_percent
                        .unwrap_or(DEFAULT_PROFIT_MARGIN_PERCENT),
                )
                .unwrap(),
                interval_operating_cost: CosmWasmCoin {
                    denom: denom.into(),
                    amount: Uint128::new(
                        args.interval_operating_cost
                            .unwrap_or(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT),
                    ),
                },
            })
        } else {
            None
        };

    let res = client
        .migrate_legacy_gateway(cost_params, None)
        .await
        .expect("failed to migrate gateway!");

    info!("migration result: {:?}", res)
}
