// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::bank::MsgSend;
use cosmrs::tx::Msg;

#[tokio::main]
async fn main() {
    let from_address = "aa".parse().unwrap();
    let to_address = "bb".parse().unwrap();
    let amount = Vec::new();

    let send_msg = MsgSend {
        from_address,
        to_address,
        amount,
    }
    .to_any();
}
