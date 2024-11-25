// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::trace;
use nym_credential_proxy_requests::api::v1::ticketbook::models::TicketbookRequest;
use nym_credentials_interface::{
    generate_keypair_user, withdrawal_request, Base58, SecretKeyUser, TicketType,
};
use nym_ecash_time::{ecash_default_expiration_date, EcashTime};
use serde::{Deserialize, Serialize};
use std::io::stdout;
use time::macros::format_description;
use time::Date;
use zeroize::Zeroizing;

fn parse_date(raw: &str) -> Result<Date, time::error::Parse> {
    let format = format_description!("[year]-[month]-[day]");
    Date::parse(raw, &format)
}

#[derive(Serialize, Deserialize)]
pub struct Bs58EncodedOutput {
    pub ecash_proxy_request: TicketbookRequest,
    pub ecash_secret: String,

    /// Needed to later unblind shares
    pub ecash_request_info_bs58: String,
}

#[derive(Debug, Parser)]
pub struct Args {
    /// Specify which type of ticketbook
    #[clap(long, default_value_t = TicketType::V1MixnetEntry)]
    pub(crate) ticketbook_type: TicketType,

    /// Set expiration date for the ticketbook
    #[clap(long, value_parser = parse_date, default_value_t = ecash_default_expiration_date())]
    pub(crate) expiration_date: Date,

    /// Provide ecash secret key (or generate a fresh one)
    #[clap(long)]
    pub(crate) ecash_secret_key_bs58: Option<String>,
}

pub async fn generate_withdrawal_request(args: Args) -> anyhow::Result<()> {
    trace!("args: {args:?}");

    let ecash_keypair = if let Some(secret_key) = args.ecash_secret_key_bs58 {
        let secret_key = Zeroizing::new(bs58::decode(Zeroizing::new(secret_key)).into_vec()?);
        let sk = SecretKeyUser::from_bytes(&secret_key)?;
        sk.into()
    } else {
        generate_keypair_user()
    };

    let (withdrawal_request, request_info) = withdrawal_request(
        ecash_keypair.secret_key(),
        args.expiration_date.ecash_unix_timestamp(),
        args.ticketbook_type.encode(),
    )?;

    let encoded = Bs58EncodedOutput {
        ecash_proxy_request: TicketbookRequest {
            withdrawal_request: withdrawal_request.into(),
            ecash_pubkey: ecash_keypair.public_key(),
            expiration_date: args.expiration_date,
            ticketbook_type: args.ticketbook_type,
            is_freepass_request: false,
        },
        ecash_secret: ecash_keypair.secret_key().to_bs58(),
        ecash_request_info_bs58: request_info.to_bs58(),
    };

    serde_json::to_writer_pretty(stdout(), &encoded)?;

    Ok(())
}
