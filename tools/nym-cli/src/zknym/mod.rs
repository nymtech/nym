use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    coconut: nym_cli_commands::zknym::Zknym,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match coconut.command {
        nym_cli_commands::zknym::ZknymCommands::IssueCredits(args) => {
            nym_cli_commands::zknym::issue_credits::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await?
        }
        nym_cli_commands::zknym::ZknymCommands::RecoverCredits(args) => {
            nym_cli_commands::zknym::recover_credits::execute(
                args,
                create_query_client(network_details)?,
            )
            .await?
        }
        nym_cli_commands::zknym::ZknymCommands::ImportCredits(args) => {
            nym_cli_commands::zknym::import_credits::execute(args).await?
        }
    }
    Ok(())
}
