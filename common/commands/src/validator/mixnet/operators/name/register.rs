use clap::Parser;
use log::{error, info};
use nym_name_service_common::{Address, Coin, NymName};
use nym_validator_client::nyxd::{error::NyxdError, traits::NameServiceSigningClient};
use tap::TapFallible;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// Name alias
    #[clap(long)]
    pub name: String,

    /// Nym address that the alias is pointing to
    #[clap(long)]
    pub nym_address: String,

    /// Deposit to be made to the service provider directory, in curent DENOMINATION (e.g. 'unym')
    #[clap(long)]
    pub deposit: u128,
}

pub async fn register(args: Args, client: SigningClient) -> Result<(), NyxdError> {
    info!(
        "Registering name alias '{}' for nym address '{}'",
        args.name, args.nym_address
    );

    let name = NymName::new(&args.name).expect("invalid name");
    let address = Address::new(&args.nym_address);

    let denom = client.current_chain_details().mix_denom.base.as_str();
    let deposit = Coin::new(args.deposit, denom);

    let res = client
        .register_name(name, address, deposit.into(), None)
        .await
        .tap_err(|err| error!("Failed to register name: {err:#?}"))?;

    info!("Registered name: {res:?}");
    Ok(())
}
