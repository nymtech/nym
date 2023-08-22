use nym_cli_commands::context::{create_signing_client, ClientArgs};
use nym_network_defaults::NymNetworkDetails;

pub(crate) async fn execute(
    global_args: ClientArgs,
    name: nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsName,
    network_details: &NymNetworkDetails,
) -> anyhow::Result<()> {
    let res = match name.command {
        nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsNameCommands::Register(register) => {
            let res = nym_cli_commands::validator::mixnet::operators::name::register::register(register, create_signing_client(global_args, network_details)?).await;
            match res {
                Ok(_) => println!("Successfully registered the name"),
                Err(_) => println!("Failed to register name")
            };
            res
        },
        nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsNameCommands::Delete(delete) => {
            let res = nym_cli_commands::validator::mixnet::operators::name::delete::delete(delete, create_signing_client(global_args, network_details)?).await;
            match res {
                Ok(_) => println!("Successfully deleted the name"),
                Err(_) => println!("Failed to delete name")
            };
            res
        },
        nym_cli_commands::validator::mixnet::operators::name::MixnetOperatorsNameCommands::CreateNameRegisterPayload(args) => {
            let res = nym_cli_commands::validator::mixnet::operators::name::register_sign_payload::create_payload(args, create_signing_client(global_args, network_details)?).await;
            match res {
                Ok(_) => (),
                Err(_) => println!("Failed to create payload")
            };
            res
        }
    };
    Ok(res?)
}
