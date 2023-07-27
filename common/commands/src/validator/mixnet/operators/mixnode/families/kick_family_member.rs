// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::traits::MixnetSigningClient;
use nym_validator_client::nyxd::traits::VestingSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// The member of the family that we intend to kick
    #[arg(long)]
    pub member: identity::PublicKey,

    /// Indicates whether the family was created (and managed) via the vesting contract
    #[arg(long)]
    pub with_vesting_account: bool,
}

pub async fn kick_family_member(args: Args, client: SigningClient) {
    info!("Leave family");

    let member = args.member.to_base58_string();

    let res = if args.with_vesting_account {
        client
            .vesting_kick_family_member(member, None)
            .await
            .expect("failed to kick family member with vesting account")
    } else {
        client
            .kick_family_member(member, None)
            .await
            .expect("failed to kick family member")
    };

    info!("Family leave result: {:?}", res);
}
