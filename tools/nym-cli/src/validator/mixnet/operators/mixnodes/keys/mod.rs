// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
pub(crate) async fn execute(
    keys: nym_cli_commands::validator::mixnet::operators::mixnode::keys::MixnetOperatorsMixnodeKeys,
) -> anyhow::Result<()> {
    match keys.command {
        nym_cli_commands::validator::mixnet::operators::mixnode::keys::MixnetOperatorsMixnodeKeysCommands::DecodeMixnodeKey(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::keys::decode_mixnode_key::decode_mixnode_key(args)
        }
    }
    Ok(())
}
