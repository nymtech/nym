// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub wasm_path: PathBuf,

    #[clap(long)]
    pub memo: Option<String>,

    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn upload(client: Client, args: Args) {
    info!("Starting contract upload!");

    let mut file = std::fs::File::open(args.wasm_path).expect("failed to open the wasm blob");
    let mut data = Vec::new();

    file.read_to_end(&mut data).unwrap();

    let memo = args.memo.unwrap_or_else(|| "contract upload".to_owned());

    let res = client
        .upload(data, memo, None)
        .await
        .expect("failed to upload the contract!");

    info!("Upload result: {:?}", res);

    println!("{}", res.code_id)
}
