use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    coconut: nym_cli_commands::coconut::Coconut,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match coconut.command {
        nym_cli_commands::coconut::CoconutCommands::IssueCredentials(args) => {
            nym_cli_commands::coconut::issue_credentials::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await?
        }
        nym_cli_commands::coconut::CoconutCommands::RecoverCredentials(args) => {
            nym_cli_commands::coconut::recover_credentials::execute(
                args,
                create_query_client(network_details)?,
            )
            .await?
        }
        nym_cli_commands::coconut::CoconutCommands::ImportCredential(args) => {
            nym_cli_commands::coconut::import_credential::execute(args).await?
        }
    }
    Ok(())
}
