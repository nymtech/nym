// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_cli_commands::validator::mixnet::operators::mixnode::families::MixnetOperatorsMixnodeFamiliesCommands;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    families: nym_cli_commands::validator::mixnet::operators::mixnode::families::MixnetOperatorsMixnodeFamilies,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match families.command {
        MixnetOperatorsMixnodeFamiliesCommands::CreateFamily(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::families::create_family::create_family(
                args,
                create_signing_client(global_args, network_details)?,
            )
                .await
        }
        MixnetOperatorsMixnodeFamiliesCommands::JoinFamily(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::families::join_family::join_family(
                args,
                create_signing_client(global_args, network_details)?,
            )
                .await
        }
        MixnetOperatorsMixnodeFamiliesCommands::LeaveFamily | MixnetOperatorsMixnodeFamiliesCommands::KickFamilyMember => todo!(),

        MixnetOperatorsMixnodeFamiliesCommands::CreateFamilyJoinPermitSignPayload(args) => {
            nym_cli_commands::validator::mixnet::operators::mixnode::families::create_family_join_permit_sign_payload::create_family_join_permit_sign_payload(args, create_query_client(network_details)?).await
        }
    }
    Ok(())
}
