//! # Ephemera API
//!
//! This module contains all the types and functions available as part of Ephemera public API.
//!
//! This API is also available over HTTP.

use std::fmt::Display;

use log::{error, trace};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

use crate::api::types::{
    ApiBlock, ApiBlockBroadcastInfo, ApiBroadcastInfo, ApiCertificate, ApiEphemeraConfig,
    ApiEphemeraMessage, ApiError, ApiVerifyMessageInBlock,
};

pub(crate) mod application;
pub(crate) mod http;
pub(crate) mod types;

/// Kademlia DHT key
pub(crate) type DhtKey = Vec<u8>;

/// Kademlia DHT value
pub(crate) type DhtValue = Vec<u8>;

/// Kademlia DHT key/value pair
pub(crate) type DhtKV = (DhtKey, DhtValue);

pub(crate) type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug)]
pub(crate) enum ToEphemeraApiCmd {
    SubmitEphemeraMessage(Box<ApiEphemeraMessage>, oneshot::Sender<Result<()>>),
    QueryBlockByHeight(u64, oneshot::Sender<Result<Option<ApiBlock>>>),
    QueryBlockByHash(String, oneshot::Sender<Result<Option<ApiBlock>>>),
    QueryLastBlock(oneshot::Sender<Result<ApiBlock>>),
    QueryBlockCertificates(String, oneshot::Sender<Result<Option<Vec<ApiCertificate>>>>),
    QueryDht(DhtKey, oneshot::Sender<Result<Option<DhtKV>>>),
    StoreInDht(DhtKey, DhtValue, oneshot::Sender<Result<()>>),
    QueryEphemeraConfig(oneshot::Sender<Result<ApiEphemeraConfig>>),
    QueryBroadcastGroup(oneshot::Sender<Result<ApiBroadcastInfo>>),
    QueryBlockBroadcastInfo(
        String,
        oneshot::Sender<Result<Option<ApiBlockBroadcastInfo>>>,
    ),
    VerifyMessageInBlock(String, String, usize, oneshot::Sender<Result<bool>>),
}

impl Display for ToEphemeraApiCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToEphemeraApiCmd::SubmitEphemeraMessage(message, _) => {
                write!(f, "SubmitEphemeraMessage({message})",)
            }
            ToEphemeraApiCmd::QueryBlockByHeight(height, _) => {
                write!(f, "QueryBlockByHeight({height})",)
            }
            ToEphemeraApiCmd::QueryBlockByHash(hash, _) => write!(f, "QueryBlockByHash({hash})",),
            ToEphemeraApiCmd::QueryLastBlock(_) => write!(f, "QueryLastBlock"),
            ToEphemeraApiCmd::QueryBlockCertificates(id, _) => {
                write!(f, "QueryBlockSignatures{id}")
            }
            ToEphemeraApiCmd::QueryDht(_, _) => {
                write!(f, "QueryDht")
            }
            ToEphemeraApiCmd::StoreInDht(_, _, _) => {
                write!(f, "StoreInDht")
            }
            ToEphemeraApiCmd::QueryEphemeraConfig(_) => {
                write!(f, "EphemeraConfig")
            }
            ToEphemeraApiCmd::QueryBroadcastGroup(_) => {
                write!(f, "BroadcastGroup")
            }
            ToEphemeraApiCmd::QueryBlockBroadcastInfo(hash, ..) => {
                write!(f, "BlockBroadcastInfo({hash})")
            }
            ToEphemeraApiCmd::VerifyMessageInBlock(block_id, message_id, height, _) => {
                write!(
                    f,
                    "VerifyMessageInBlock({block_id}, {message_id}, {height})",
                )
            }
        }
    }
}

pub(crate) struct ApiListener {
    pub(crate) messages_rcv: Receiver<ToEphemeraApiCmd>,
}

impl ApiListener {
    pub(crate) fn new(messages_rcv: Receiver<ToEphemeraApiCmd>) -> Self {
        Self { messages_rcv }
    }
}

#[derive(Clone)]
pub struct CommandExecutor {
    pub(crate) commands_channel: Sender<ToEphemeraApiCmd>,
}

impl CommandExecutor {
    pub(crate) fn new() -> (CommandExecutor, ApiListener) {
        let (commands_channel, signed_messages_rcv) = channel(100);
        let api_listener = ApiListener::new(signed_messages_rcv);
        let api = CommandExecutor { commands_channel };
        (api, api_listener)
    }

    /// Returns block with given id if it exists
    ///
    /// # Arguments
    /// * `block_id` - Block id
    ///
    /// # Returns
    /// * `ApiBlock` - Block
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn get_block_by_id(&self, block_id: String) -> Result<Option<ApiBlock>> {
        trace!("get_block_by_id({:?})", block_id);
        self.send_and_wait_response(|tx| ToEphemeraApiCmd::QueryBlockByHash(block_id, tx))
            .await
    }

    /// Returns block with given height if it exists
    ///
    /// # Arguments
    /// * `height` - Block height
    ///
    /// # Returns
    /// * `ApiBlock` - Block
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn get_block_by_height(&self, height: u64) -> Result<Option<ApiBlock>> {
        trace!("get_block_by_height({:?})", height);
        self.send_and_wait_response(|tx| ToEphemeraApiCmd::QueryBlockByHeight(height, tx))
            .await
    }

    /// Returns last block. Which has maximum height and is stored in database
    ///
    /// # Returns
    /// * `ApiBlock` - Last block
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn get_last_block(&self) -> Result<ApiBlock> {
        trace!("get_last_block()");
        self.send_and_wait_response(ToEphemeraApiCmd::QueryLastBlock)
            .await
    }

    /// Returns signatures for given block id
    ///
    /// # Arguments
    /// * `block_hash` - Block id
    ///
    /// # Returns
    /// * `Vec<ApiCertificate>` - Certificates
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn get_block_certificates(
        &self,
        block_hash: String,
    ) -> Result<Option<Vec<ApiCertificate>>> {
        trace!("get_block_certificates({block_hash:?})",);
        self.send_and_wait_response(|tx| ToEphemeraApiCmd::QueryBlockCertificates(block_hash, tx))
            .await
    }

    /// Queries DHT for given key
    ///
    /// # Arguments
    /// * `key` - DHT key
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    ///
    /// # Returns
    /// * `Some((key, value))` - If key is found
    /// * `None` - If key is not found
    pub async fn query_dht(&self, key: DhtKey) -> Result<Option<(DhtKey, DhtValue)>> {
        trace!("get_dht({key:?})");
        //TODO: this needs timeout(somewhere around dht query functionality)
        self.send_and_wait_response(|tx| ToEphemeraApiCmd::QueryDht(key, tx))
            .await
    }

    /// Stores given key-value pair in DHT
    ///
    /// # Arguments
    /// * `key` - DHT key
    /// * `value` - DHT value
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn store_in_dht(&self, key: DhtKey, value: DhtValue) -> Result<()> {
        trace!("store_in_dht({key:?}, {value:?})");
        self.send_and_wait_response(|tx| ToEphemeraApiCmd::StoreInDht(key, value, tx))
            .await
    }

    /// Returns node configuration
    ///
    /// # Returns
    /// * `ApiEphemeraConfig` - Node configuration
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn get_node_config(&self) -> Result<ApiEphemeraConfig> {
        trace!("get_node_config()");
        self.send_and_wait_response(ToEphemeraApiCmd::QueryEphemeraConfig)
            .await
    }

    /// Returns broadcast group
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    ///
    /// # Return
    /// * `ApiBroadcastInfo` - Broadcast group
    pub async fn get_broadcast_info(&self) -> Result<ApiBroadcastInfo> {
        trace!("get_broadcast_group()");
        self.send_and_wait_response(ToEphemeraApiCmd::QueryBroadcastGroup)
            .await
    }

    /// Returns block broadcast info.
    ///
    /// # Arguments
    /// * `block_hash` - Block hash
    ///
    /// # Return
    /// * `ApiBlockBroadcastInfo` - Block broadcast info
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn get_block_broadcast_info(
        &self,
        block_hash: String,
    ) -> Result<Option<ApiBlockBroadcastInfo>> {
        trace!("get_broadcast_group()");
        self.send_and_wait_response(|tx| ToEphemeraApiCmd::QueryBlockBroadcastInfo(block_hash, tx))
            .await
    }

    /// Send a message to Ephemera which should then be included in mempool  and broadcast to all peers
    ///
    /// # Arguments
    /// * `message` - Message to be sent
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn send_ephemera_message(&self, message: ApiEphemeraMessage) -> Result<()> {
        trace!("send_ephemera_message({message})",);
        self.send_and_wait_response(|tx| {
            ToEphemeraApiCmd::SubmitEphemeraMessage(message.into(), tx)
        })
        .await
    }

    /// Verifies if given message is in block identified by block hash
    /// Returns true if message is in block, false otherwise. False can also mean that block or message
    /// does not exist.
    ///
    /// # Arguments
    /// * `request` - Message and block hash
    ///
    /// # Errors
    /// * `ApiError::InternalError` - If there is an internal error
    pub async fn verify_message_in_block(&self, request: ApiVerifyMessageInBlock) -> Result<bool> {
        trace!("verify_message_in_block({request})",);
        let block_hash = request.block_hash;
        let message_hash = request.message_hash;
        let index = request.message_index;
        self.send_and_wait_response(|tx| {
            ToEphemeraApiCmd::VerifyMessageInBlock(block_hash, message_hash, index, tx)
        })
        .await
    }

    async fn send_and_wait_response<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(oneshot::Sender<Result<R>>) -> ToEphemeraApiCmd,
        R: Send + 'static,
    {
        let (tx, rcv) = oneshot::channel();
        let cmd = f(tx);
        if let Err(e) = self.commands_channel.send(cmd).await {
            error!("Failed to send command to Ephemera: {e:?}",);
            return Err(ApiError::Internal(
                "Failed to receive response from Ephemera".to_string(),
            ));
        }
        rcv.await.map_err(|e| {
            error!("Failed to receive response from Ephemera: {e:?}",);
            ApiError::Internal("Failed to receive response from Ephemera".to_string())
        })?
    }
}
