use nym_network_defaults::NymNetworkDetails;
use nym_cli_commands::context::ClientArgs;

pub(crate) mod delegators;
pub(crate) mod operators;
pub(crate) mod query;

pub(crate) async fn execute(
    global_args: ClientArgs,
    mixnet: nym_cli_commands::validator::mixnet::Mixnet,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match mixnet.command {
        nym_cli_commands::validator::mixnet::MixnetCommands::Delegators(delegators) => {
            delegators::execute(global_args, delegators, network_details).await?
        }
        nym_cli_commands::validator::mixnet::MixnetCommands::Operators(operators) => {
            operators::execute(global_args, operators, network_details).await?
        }
        nym_cli_commands::validator::mixnet::MixnetCommands::Query(query) => {
            query::execute(query, network_details).await?
        }
    }
    Ok(())
}
