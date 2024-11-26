// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::ClientArgs;
use nym_cli_commands::validator::mixnet::operators::MixnetOperatorsCommands;
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod gateways;
pub(crate) mod identity_key;
pub(crate) mod mixnodes;
mod nymnodes;

pub(crate) async fn execute(
    global_args: ClientArgs,
    operators: nym_cli_commands::validator::mixnet::operators::MixnetOperators,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match operators.command {
        MixnetOperatorsCommands::Nymnode(nymnode) => {
            nymnodes::execute(global_args, nymnode, network_details).await
        }
        MixnetOperatorsCommands::Gateway(gateway) => {
            gateways::execute(global_args, gateway, network_details).await
        }
        MixnetOperatorsCommands::Mixnode(mixnode) => {
            mixnodes::execute(global_args, mixnode, network_details).await
        }
        MixnetOperatorsCommands::IdentityKey(identity_key) => {
            identity_key::execute(global_args, identity_key, network_details).await
        }
    }
}
