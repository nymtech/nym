use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    coconut: nym_cli_commands::coconut::Ecash,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match coconut.command {
        nym_cli_commands::coconut::EcashCommands::IssueTicketBook(args) => {
            nym_cli_commands::coconut::issue_ticket_book::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await?
        }
        nym_cli_commands::coconut::EcashCommands::RecoverTicketBook(args) => {
            nym_cli_commands::coconut::recover_ticket_book::execute(
                args,
                create_query_client(network_details)?,
            )
            .await?
        }
        nym_cli_commands::coconut::EcashCommands::ImportTicketBook(args) => {
            nym_cli_commands::coconut::import_ticket_book::execute(args).await?
        }
    }
    Ok(())
}
