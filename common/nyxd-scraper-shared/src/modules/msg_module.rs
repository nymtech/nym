// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::{DecodedMessage, ParsedTransactionDetails};
use crate::error::ScraperError;
use async_trait::async_trait;
use cosmrs::Any;
use cosmrs::tx::Msg;

/// Parse a protobuf `Any` message into a strongly typed Cosmos message.
///
/// # Example
///
/// ```rust,ignore
/// let execute_msg: MsgExecuteContract = parse_msg(msg)?;
/// ```
///
/// # Errors
///
/// Returns `ScraperError::MsgParseFailure` if:
/// - The type URL doesn't match the expected type
/// - The protobuf bytes are malformed
/// - The message schema is incompatible with this version of the code
pub fn parse_msg<T: Msg>(msg: &Any) -> Result<T, ScraperError> {
    T::from_any(msg).map_err(|source| ScraperError::MsgParseFailure {
        type_url: msg.type_url.clone(),
        source,
    })
}

/// Trait for modules that process specific message types from blockchain transactions.
///
/// # Parameters
///
/// - `index`: Position of this message within the transaction (0-based)
/// - `msg`: Raw protobuf message (use `parse_msg()` to decode)
/// - `decoded_msg`: Pre-decoded JSON representation (may be None for unsupported types)
/// - `tx`: Transaction details including block height, hash, and execution result
///
/// # Error Handling
///
/// - Return `Err` for critical failures that should stop block processing
/// - Return `Ok(())` for non-critical errors (e.g., unexpected contract schema)
/// - Log warnings for debugging without propagating errors
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
