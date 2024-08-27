// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use cosmwasm_std::Uint128;
use log::info;
use nym_mixnet_contract_common::{MixId, MixNodeCostParams, Percent};
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use nym_validator_client::nyxd::CosmWasmCoin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(
        long,
        help = "input your profit margin as follows; (so it would be 20, rather than 0.2)"
    )]
    pub profit_margin_percent: Option<u8>,

    #[clap(
        long,
        help = "operating cost in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub interval_operating_cost: Option<u128>,
}

pub async fn update_cost_params(args: Args, client: SigningClient, mix_id: MixId) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    fn convert_to_percent(value: u64) -> Percent {
        Percent::from_percentage_value(value).expect("Invalid value")
    }

    let default_profit_margin: Percent = convert_to_percent(20);

    let profit_margin_percent = match client.get_mixnode_rewarding_details(mix_id).await {
        Ok(details) => details
            .rewarding_details
            .map(|rd| rd.cost_params.profit_margin_percent)
            .unwrap_or(default_profit_margin),
        Err(_) => {
            eprintln!("Failed to obtain profit margin from node, using default value of 20%");
            default_profit_margin
        }
    };

    let profit_margin_value = args
        .profit_margin_percent
        .map(|pm| convert_to_percent(pm as u64))
        .unwrap_or(profit_margin_percent);

    let cost_params = MixNodeCostParams {
        profit_margin_percent: profit_margin_value,
        interval_operating_cost: CosmWasmCoin {
            denom: denom.into(),
            amount: Uint128::new(args.interval_operating_cost.unwrap_or(40_000_000)),
        },
    };

    info!("Starting mixnode params updating!");
    let res = client
        .update_mixnode_cost_params(cost_params, None)
        .await
        .expect("failed to update cost params");

    info!("Cost params result: {:?}", res)
}
