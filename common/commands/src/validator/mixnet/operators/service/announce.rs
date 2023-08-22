use clap::Parser;
use log::info;
use nym_contracts_common::signing::MessageSignature;
use nym_service_provider_directory_common::{Coin, NymAddress, ServiceDetails, ServiceType};
use nym_validator_client::nyxd::contract_traits::SpDirectorySigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub nym_address: String,

    #[clap(long)]
    pub signature: MessageSignature,

    /// Deposit to be made to the service provider directory, in curent DENOMINATION (e.g. 'unym')
    #[clap(long)]
    pub deposit: u128,

    #[clap(long)]
    pub identity_key: String,
}

pub async fn announce(args: Args, client: SigningClient) {
    info!("Annoucing service provider");

    let nym_address = NymAddress::Address(args.nym_address);
    let service_type = ServiceType::NetworkRequester;
    let service = ServiceDetails {
        nym_address,
        service_type,
        identity_key: args.identity_key,
    };

    let denom = client.current_chain_details().mix_denom.base.as_str();
    let deposit = Coin::new(args.deposit, denom);

    let res = client
        .announce_service_provider(service, args.signature, deposit.into(), None)
        .await
        .expect("Failed to announce service provider");

    info!("Announced service provider: {res:?}");
}
