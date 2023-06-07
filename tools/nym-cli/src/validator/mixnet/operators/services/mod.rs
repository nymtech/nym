use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    service: nym_cli_commands::validator::mixnet::operators::service::MixnetOperatorsService,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match service.command {
        nym_cli_commands::validator::mixnet::operators::service::MixnetOperatorsServiceCommands::Announce(announce) => nym_cli_commands::validator::mixnet::operators::service::announce::announce(announce, create_signing_client(global_args, network_details)?).await,
        nym_cli_commands::validator::mixnet::operators::service::MixnetOperatorsServiceCommands::Delete(delete) => nym_cli_commands::validator::mixnet::operators::service::delete::delete(delete, create_signing_client(global_args, network_details)?).await,
        nym_cli_commands::validator::mixnet::operators::service::MixnetOperatorsServiceCommands::CreateServiceAnnounceSignPayload(args) => nym_cli_commands::validator::mixnet::operators::service::announce_sign_payload::create_payload(args, create_signing_client(global_args, network_details)?).await,
    }
    Ok(())
}
