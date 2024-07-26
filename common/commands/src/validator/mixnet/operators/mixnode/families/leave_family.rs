// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// The head of the family that we intend to leave
    #[arg(long)]
    pub family_head: identity::PublicKey,
}

pub async fn leave_family(args: Args, client: SigningClient) {
    info!("Leave family");

    let family_head = FamilyHead::new(args.family_head.to_base58_string());

    let res = client
        .leave_family(family_head, None)
        .await
        .expect("failed to leave family");

    info!("Family leave result: {:?}", res);
}
