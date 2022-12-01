pub(crate) async fn execute(
    cosmwasm: nym_cli_commands::validator::cosmwasm::generators::GenerateMessage,
) -> anyhow::Result<()> {
    match cosmwasm.command {
        nym_cli_commands::validator::cosmwasm::generators::GenerateMessageCommands::Mixnet(
            args,
        ) => nym_cli_commands::validator::cosmwasm::generators::mixnet::generate(args).await,

        nym_cli_commands::validator::cosmwasm::generators::GenerateMessageCommands::Vesting(
            args,
        ) => nym_cli_commands::validator::cosmwasm::generators::vesting::generate(args).await,
    }
    Ok(())
}
