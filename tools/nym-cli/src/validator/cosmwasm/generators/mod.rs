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

        nym_cli_commands::validator::cosmwasm::generators::GenerateMessageCommands::EcashBandwidth(
            args,
        ) => nym_cli_commands::validator::cosmwasm::generators::ecash_bandwidth::generate(args).await,

        nym_cli_commands::validator::cosmwasm::generators::GenerateMessageCommands::CoconutDKG(
            args,
        ) => nym_cli_commands::validator::cosmwasm::generators::coconut_dkg::generate(args).await,

        nym_cli_commands::validator::cosmwasm::generators::GenerateMessageCommands::Multisig(
            args,
        ) => nym_cli_commands::validator::cosmwasm::generators::multisig::generate(args).await,
    }
    Ok(())
}
