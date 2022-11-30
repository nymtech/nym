pub(crate) async fn execute(
    cosmwasm: nym_cli_commands::validator::cosmwasm::generators::GenerateMessage,
) -> anyhow::Result<()> {
    match cosmwasm.command {
        nym_cli_commands::validator::cosmwasm::generators::GenerateMessageCommands::Mixnode(
            args,
        ) => nym_cli_commands::validator::cosmwasm::generators::mixnode::generate(args).await,
    }
    Ok(())
}
