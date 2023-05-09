use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    name: nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsName,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match name.command {
        nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsNameCommands::Register(register) => nym_cli_commands::validator::mixnet::operators::name::register::register(register, create_signing_client(global_args, network_details)?).await,
        nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsNameCommands::Delete(delete) => nym_cli_commands::validator::mixnet::operators::name::delete::delete(delete, create_signing_client(global_args, network_details)?).await,
    }
    Ok(())
}
