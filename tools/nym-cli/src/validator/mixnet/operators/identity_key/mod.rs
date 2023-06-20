use nym_cli_commands::context::ClientArgs;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    _global_args: ClientArgs,
    identity_key: nym_cli_commands::validator::mixnet::operators::identity_key::MixnetOperatorsIdentityKey,
    _network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    let res = match identity_key.command {
        nym_cli_commands::validator::mixnet::operators::identity_key::MixnetOperatorsIdentityKeyCommands::Sign(sign_args) => {
            nym_cli_commands::validator::mixnet::operators::identity_key::sign(sign_args).await
        }
    };
    Ok(res?)
}
