// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_refresh_gateway_keys::{
    refresh_gateway_keys, CommonClientRefreshGatewayKeysArgs,
};
use nym_client_core::cli_helpers::database_key::DatabaseKeyArgs;
use nym_client_core::cli_helpers::key_passphrase::KeyPassphraseArgs;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientRefreshGatewayKeysArgs,

    #[command(flatten)]
    key_passphrase: KeyPassphraseArgs,

    #[command(flatten)]
    database_key: DatabaseKeyArgs,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl AsRef<CommonClientRefreshGatewayKeysArgs> for Args {
    fn as_ref(&self) -> &CommonClientRefreshGatewayKeysArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(mut args: Args) -> Result<(), Socks5ClientError> {
    let key_passphrase = args.key_passphrase.key_passphrase.take();
    let res = refresh_gateway_keys::<CliSocks5Client, _>(
        &args,
        key_passphrase,
        args.database_key.database_key.as_ref(),
    )
    .await?;

    println!("{}", args.output.format(&res));
    Ok(())
}
