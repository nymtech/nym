// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;

use nym_mixnet_contract_common::{Coin, NodeId};
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::contract_traits::VestingSigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub mix_id: Option<NodeId>,

    #[clap(long)]
    pub identity_key: Option<String>,

    #[clap(long)]
    pub on_behalf_of: Option<String>,

    #[clap(long)]
    pub amount: u128,
}

pub async fn vesting_delegate_to_mixnode(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!("Starting vesting delegation to mixnode");

    let mix_id = match args.mix_id {
        Some(mix_id) => mix_id,
        None => {
            let identity_key = args
                .identity_key
                .expect("either mix_id or mix_identity has to be specified");
            let node_details = client
                .get_mixnode_details_by_identity(identity_key)
                .await
                .expect("contract query failed")
                .mixnode_details
                .expect("mixnode with the specified identity doesnt exist");
            node_details.mix_id()
        }
    };

    let coin = Coin::new(args.amount, denom);

    let res = client
        .vesting_delegate_to_mixnode(mix_id, coin.into(), args.on_behalf_of, None)
        .await
        .expect("failed to delegate to mixnode!");

    info!("vesting delegating to mixnode: {:?}", res);
}
