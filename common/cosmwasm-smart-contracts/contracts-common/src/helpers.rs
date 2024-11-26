// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Response};

pub trait ResponseExt<T> {
    fn add_optional_message(self, msg: Option<impl Into<CosmosMsg<T>>>) -> Self;

    fn send_tokens(self, to: impl AsRef<str>, amount: Coin) -> Self;
}

impl<T> ResponseExt<T> for Response<T> {
    fn add_optional_message(self, msg: Option<impl Into<CosmosMsg<T>>>) -> Self {
        if let Some(msg) = msg {
            self.add_message(msg)
        } else {
            self
        }
    }

    fn send_tokens(self, to: impl AsRef<str>, amount: Coin) -> Self {
        self.add_message(BankMsg::Send {
            to_address: to.as_ref().to_string(),
            amount: vec![amount],
        })
    }
}
