use clap::Parser;
use log::info;
use nym_service_provider_directory_common::ServiceId;
use nym_validator_client::nyxd::contract_traits::SpDirectorySigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub id: ServiceId,
}

pub async fn delete(args: Args, client: SigningClient) {
    info!("Deleting service provider with id {}", args.id);

    let res = client
        .delete_service_provider_by_id(args.id, None)
        .await
        .expect("Failed to delete service provider");

    info!("Deleted: {res:?}");
}
