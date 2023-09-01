use log::trace;
use thiserror::Error;

use crate::api::types::{ApiBlock, ApiEphemeraMessage};

#[derive(Debug, Clone, PartialEq)]
pub enum RemoveMessages {
    /// Remove all messages from the mempool
    All,
    /// Remove only inclued messages from the mempool
    Selected(Vec<ApiEphemeraMessage>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckBlockResult {
    /// Accept the block
    Accept,
    /// Reject the block with a reason.
    Reject,
    /// Reject the block and remove messages from the mempool
    RejectAndRemoveMessages(RemoveMessages),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckBlockResponse {
    pub accept: bool,
    pub reason: Option<String>,
}

#[derive(Error, Debug)]
pub enum Error {
    //Just a placeholder for now
    #[error("ApplicationError: {0}")]
    Application(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Cosmos style ABCI application hook
///
/// These functions should be relatively fast, as they are called synchronously by Ephemera main loop.
pub trait Application {
    /// It's called when receiving a new message from network before adding it to the mempool.
    /// It's up to the application to decide whether the message is valid or not.
    /// Basic check could be for example signature verification.
    ///
    /// # Arguments
    /// * `message` - message to be checked
    ///
    /// # Returns
    /// * `true` - if the message is valid
    /// * `false` - if the message is invalid
    ///
    /// # Errors
    /// * `Error::General` - if there was an error during validation
    fn check_tx(&self, message: ApiEphemeraMessage) -> Result<bool>;

    /// Ephemera produces new blocks with configured interval.
    /// Application can decide whether to accept the block or not.
    /// For example, if the block doesn't contain any transactions, it can be rejected.
    ///
    /// # Arguments
    /// * `block` - block to be checked
    ///
    /// # Returns
    /// * `CheckBlockResult::Accept` - if the block is valid
    /// * `CheckBlockResult::Reject` - if the block is invalid
    /// * `CheckBlockResult::RejectAndRemoveMessages` - if the block is invalid and some messages should be removed from the mempool
    ///
    /// # Errors
    /// * `Error::General` - if there was an error during validation
    fn check_block(&self, block: &ApiBlock) -> Result<CheckBlockResult>;

    /// Deliver Block is called after block is confirmed by Ephemera and persisted to the storage.
    ///
    /// # Arguments
    /// * `block` - block to be delivered
    ///
    /// # Errors
    /// * `Error::General` - if there was an error during validation
    fn deliver_block(&self, block: ApiBlock) -> Result<()>;
}

/// Dummy application which doesn't do any validation.
/// Might be useful for testing.
#[derive(Default)]
pub struct Dummy;

impl Application for Dummy {
    fn check_tx(&self, tx: ApiEphemeraMessage) -> Result<bool> {
        trace!("check_tx: {tx:?}");
        Ok(true)
    }

    fn check_block(&self, block: &ApiBlock) -> Result<CheckBlockResult> {
        trace!("accept_block: {block:?}");
        Ok(CheckBlockResult::Accept)
    }

    fn deliver_block(&self, block: ApiBlock) -> Result<()> {
        trace!("deliver_block: {block:?}");
        Ok(())
    }
}
