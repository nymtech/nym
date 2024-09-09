use nym_cli_commands::context::{create_query_client, create_signing_client, ClientArgs};
use nym_cli_commands::ecash::EcashCommands;
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    coconut: nym_cli_commands::ecash::Ecash,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    match coconut.command {
        EcashCommands::IssueTicketBook(args) => {
            nym_cli_commands::ecash::issue_ticket_book::execute(
                args,
                create_signing_client(global_args, network_details)?,
            )
            .await?
        }
        EcashCommands::RecoverTicketBook(args) => {
            nym_cli_commands::ecash::recover_ticket_book::execute(
                args,
                create_query_client(network_details)?,
            )
            .await?
        }
        EcashCommands::ImportTicketBook(args) => {
            nym_cli_commands::ecash::import_ticket_book::execute(args).await?
        }
        EcashCommands::ImportCoinIndexSignatures(args) => {
            nym_cli_commands::ecash::import_coin_index_signatures::execute(args).await?
        }
        EcashCommands::ImportExpirationDateSignatures(args) => {
            nym_cli_commands::ecash::import_expiration_date_signatures::execute(args).await?
        }
        EcashCommands::ImportMasterVerificationKey(args) => {
            nym_cli_commands::ecash::import_master_verification_key::execute(args).await?
        }
    }
    Ok(())
}
