use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) mod generators;

pub(crate) async fn execute(
    global_args: ClientArgs,
    cosmwasm: nym_cli_commands::validator::cosmwasm::Cosmwasm,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match cosmwasm.command {
        nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Upload(args) => {
            nym_cli_commands::validator::cosmwasm::upload_contract::upload(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Init(args) => {
            nym_cli_commands::validator::cosmwasm::init_contract::init(
                args,
                create_signing_client(global_args, network_details)?,
                network_details,
            )
            .await
        }

        nym_cli_commands::validator::cosmwasm::CosmwasmCommands::GenerateInitMessage(generator) => {
            generators::execute(generator).await?
        }
        nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Migrate(args) => {
            nym_cli_commands::validator::cosmwasm::migrate_contract::migrate(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        nym_cli_commands::validator::cosmwasm::CosmwasmCommands::Execute(args) => {
            nym_cli_commands::validator::cosmwasm::execute_contract::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await
        }
        nym_cli_commands::validator::cosmwasm::CosmwasmCommands::RawContractState(args) => {
            nym_cli_commands::validator::cosmwasm::raw_contract_state::execute(
                args,
                create_query_client(network_details)?,
            )
            .await?
        }
    }
    Ok(())
}
