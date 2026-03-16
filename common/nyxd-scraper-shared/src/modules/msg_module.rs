// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::{DecodedMessage, ParsedTransactionDetails};
use crate::error::ScraperError;
use async_trait::async_trait;
use cosmrs::Any;
use cosmrs::tx::Msg;

pub fn parse_msg<T: Msg>(msg: &Any) -> Result<T, ScraperError> {
    T::from_any(msg).map_err(|source| ScraperError::MsgParseFailure {
        type_url: msg.type_url.clone(),
        source,
    })
}

#[async_trait]
pub trait MsgModule {
    fn type_url(&self) -> String;

    async fn handle_msg(
        &mut self,
        index: usize,
        msg: &Any,
        decoded_msg: &DecodedMessage,
        tx: &ParsedTransactionDetails,
    ) -> Result<(), ScraperError>;
}
