// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::validator::mixnet::operators::nymnode;
use clap::Parser;

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
    // the below can handle both, nymnode and legacy mixnode
    nymnode::settings::update_cost_params::update_cost_params(
        nymnode::settings::update_cost_params::Args {
            profit_margin_percent: args.profit_margin_percent,
            interval_operating_cost: args.interval_operating_cost,
        },
        client,
    )
    .await
}
