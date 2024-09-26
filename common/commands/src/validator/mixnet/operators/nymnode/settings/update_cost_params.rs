// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use anyhow::anyhow;
use clap::Parser;
use cosmwasm_std::Uint128;
use log::info;
use nym_mixnet_contract_common::{
    NodeCostParams, Percent, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, DEFAULT_PROFIT_MARGIN_PERCENT,
};
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use nym_validator_client::nyxd::CosmWasmCoin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(
        long,
        help = "input your profit margin as follows; (so it would be 20, rather than 0.2)"
    )]
    pub profit_margin_percent: Option<u64>,

    #[clap(
        long,
        help = "operating cost in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub interval_operating_cost: Option<u128>,
}

pub async fn update_cost_params(args: Args, client: SigningClient) -> anyhow::Result<()> {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    let default_profit_margin =
        Percent::from_percentage_value(DEFAULT_PROFIT_MARGIN_PERCENT).unwrap();

    let node_details = client
        .get_owned_nymnode(&client.address())
        .await?
        .details
        .ok_or_else(|| anyhow!("the client does not own any nodes"))?;
    let current_parameters = node_details.rewarding_details.cost_params;

    let profit_margin_percent = current_parameters
        .map(|rd| rd.cost_params.profit_margin_percent)
        .unwrap_or(default_profit_margin);

    let profit_margin_value = args
        .profit_margin_percent
        .map(|pm| Percent::from_percentage_value(pm as u64))
        .unwrap_or(profit_margin_percent)?;

    let cost_params = NodeCostParams {
        profit_margin_percent: profit_margin_value,
        interval_operating_cost: CosmWasmCoin {
            denom: denom.into(),
            amount: Uint128::new(
                args.interval_operating_cost
                    .unwrap_or(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT),
            ),
        },
    };

    info!("Starting nym node params updating!");
    let res = client
        .update_cost_params(cost_params, None)
        .await
        .expect("failed to update cost params");

    info!("Cost params result: {:?}", res);
    Ok(())
}
