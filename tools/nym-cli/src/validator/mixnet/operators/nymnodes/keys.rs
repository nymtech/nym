// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::validator::mixnet::operators::nymnode::keys::MixnetOperatorsNymNodeKeysCommands;

pub(crate) async fn execute(
    keys: nym_cli_commands::validator::mixnet::operators::nymnode::keys::MixnetOperatorsNymNodeKeys,
) -> anyhow::Result<()> {
    match keys.command {
       MixnetOperatorsNymNodeKeysCommands::DecodeNodeKey(args) => {
            nym_cli_commands::validator::mixnet::operators::nymnode::keys::decode_node_key::decode_node_key(args)
        }
    }
    Ok(())
}
