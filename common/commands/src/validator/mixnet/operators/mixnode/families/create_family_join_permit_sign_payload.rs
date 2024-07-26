// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClient;
use crate::utils::DataWrapper;
use clap::Parser;
use cosmrs::AccountId;
use log::info;
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::construct_family_join_permit;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// Account address (i.e. owner of the family head) which will be used for issuing the permit
    #[arg(long)]
    pub address: AccountId,

    // might as well validate the value when parsing the arguments
    /// Identity of the member for whom we're issuing the permit
    #[arg(long)]
    pub member: identity::PublicKey,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn create_family_join_permit_sign_payload(args: Args, client: QueryClient) {
    info!("Create family join permit sign payload");

    // get the address of our mixnode to recover the family head information
    let Some(mixnode) = client
        .get_owned_mixnode(&args.address)
        .await
        .unwrap()
        .mixnode_details
    else {
        eprintln!("{} does not seem to even own a mixnode!", args.address);
        return;
    };

    // make sure this mixnode is actually a family head
    if client
        .get_node_family_by_head(mixnode.bond_information.identity().to_string())
        .await
        .unwrap()
        .family
        .is_none()
    {
        eprintln!("{} does not even seem to own a family!", args.address);
        return;
    }

    let nonce = match client.get_signing_nonce(&args.address).await {
        Ok(nonce) => nonce,
        Err(err) => {
            eprint!(
                "failed to query for the signing nonce of {}: {err}",
                args.address
            );
            return;
        }
    };

    let head = FamilyHead::new(mixnode.bond_information.identity());

    let payload = construct_family_join_permit(nonce, head, args.member.to_base58_string());
    let wrapper = DataWrapper::new(payload.to_base58_string().unwrap());
    println!("{}", args.output.format(&wrapper))
}
