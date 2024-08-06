// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_contracts_common::signing::MessageSignature;
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// The head of the family that we intend to join
    #[arg(long)]
    pub family_head: identity::PublicKey,

    /// Permission, as provided by the family head, for joining the family
    #[arg(long)]
    pub join_permit: MessageSignature,
}

pub async fn join_family(args: Args, client: SigningClient) {
    info!("Join family");

    let family_head = FamilyHead::new(args.family_head.to_base58_string());

    let res = client
        .join_family(args.join_permit, family_head, None)
        .await
        .expect("failed to join family");

    info!("Family join result: {:?}", res);
}
