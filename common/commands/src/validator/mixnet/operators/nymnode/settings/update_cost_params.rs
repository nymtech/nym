// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use anyhow::anyhow;
use clap::Parser;
use cosmwasm_std::Uint128;
use log::info;
use nym_mixnet_contract_common::{NodeCostParams, Percent};
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

pub async fn update_cost_params(args: Args, client: SigningClient) -> anyhow::Result<()> {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    let current_parameters = if let Some(client_mixnode) = client
        .get_owned_mixnode(&client.address())
        .await?
        .mixnode_details
    {
        client_mixnode.rewarding_details.cost_params
    } else {
        client
            .get_owned_nymnode(&client.address())
            .await?
            .details
            .ok_or_else(|| anyhow!("the client does not own any nodes"))?
            .rewarding_details
            .cost_params
    };

    let profit_margin_percent = args
        .profit_margin_percent
        .map(|pm| Percent::from_percentage_value(pm as u64))
        .unwrap_or(Ok(current_parameters.profit_margin_percent))?;

    let interval_operating_cost = args
        .interval_operating_cost
        .map(|oc| CosmWasmCoin {
            denom: denom.into(),
            amount: Uint128::new(oc),
        })
        .unwrap_or(current_parameters.interval_operating_cost);

    let cost_params = NodeCostParams {
        profit_margin_percent,
        interval_operating_cost,
    };

    info!("Starting cost params updating using {cost_params:?} !");
    let res = client
        .update_cost_params(cost_params, None)
        .await
        .expect("failed to update cost params");

    info!("Cost params result: {:?}", res);
    Ok(())
}
