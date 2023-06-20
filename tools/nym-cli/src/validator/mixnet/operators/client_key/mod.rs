use nym_cli_commands::context::ClientArgs;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    _global_args: ClientArgs,
    client_key: nym_cli_commands::validator::mixnet::operators::client_key::MixnetOperatorsClientKey,
    _network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    let res = match client_key.command {
        nym_cli_commands::validator::mixnet::operators::client_key::MixnetOperatorsClientKeyCommands::Sign(sign_args) => {
            nym_cli_commands::validator::mixnet::operators::client_key::sign(sign_args).await
        }
    };
    Ok(res?)
}
