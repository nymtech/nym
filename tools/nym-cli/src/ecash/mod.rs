use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    coconut: nym_cli_commands::ecash::Ecash,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match coconut.command {
        nym_cli_commands::ecash::EcashCommands::IssueTicketBook(args) => {
            nym_cli_commands::ecash::issue_ticket_book::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await?
        }
        nym_cli_commands::ecash::EcashCommands::RecoverTicketBook(args) => {
            nym_cli_commands::ecash::recover_ticket_book::execute(
                args,
                create_query_client(network_details)?,
            )
            .await?
        }
        nym_cli_commands::ecash::EcashCommands::ImportTicketBook(args) => {
            nym_cli_commands::ecash::import_ticket_book::execute(args).await?
        }
        nym_cli_commands::ecash::EcashCommands::GenerateTicket(args) => {
            nym_cli_commands::ecash::generate_ticket::execute(args).await?
        }
    }
    Ok(())
}
