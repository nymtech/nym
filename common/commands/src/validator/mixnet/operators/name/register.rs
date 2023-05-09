use clap::Parser;
use log::info;
use nym_name_service_common::{Coin, NymName, Address};
use nym_validator_client::nyxd::traits::NameServiceSigningClient;

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

pub async fn register(args: Args, client: SigningClient) {
    info!("Registering name alias for nym address");

    let name = NymName::new(&args.name).expect("invalid name");
    let address = Address::new(&args.nym_address);

    let denom = client.current_chain_details().mix_denom.base.as_str();
    let deposit = Coin::new(args.deposit, denom);

    let res = client
        .register_name(name, address, deposit.into(), None)
        .await
        .expect("Failed to announce service provider");

    info!("Announced service provider: {res:?}");
}
