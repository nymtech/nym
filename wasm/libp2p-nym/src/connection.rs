// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! Connection (StreamMuxer) implementation for multiplexing substreams over Nym.

use futures::channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use futures::StreamExt;
use libp2p::core::{muxing::StreamMuxerEvent, PeerId, StreamMuxer};
use log::debug;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use nym_wasm_utils::console_log;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use super::error::Error;
use super::message::{
    ConnectionId, Message, OutboundMessage, SubstreamId, SubstreamMessage, SubstreamMessageType,
    TransportMessage,
};
use super::substream::Substream;

/// Connection represents the result of a connection setup process.
/// It implements `StreamMuxer` and thus has stream multiplexing built in.
#[derive(Debug)]
pub struct Connection {
    pub(crate) peer_id: PeerId,
    /// This will be Some(Recipient) for dialing connections since the outbound conn knows the nym/ multiaddr of the recipient, whereas receivers of connection requests will reply with SURBs
    pub(crate) remote_recipient: Option<Recipient>,
    pub(crate) id: ConnectionId,

    /// receive inbound messages from the `InnerConnection`
    pub(crate) inbound_rx: UnboundedReceiver<SubstreamMessage>,

    /// substream ID -> outbound pending substream exists
    /// the key is deleted when the response is received, or the request times out
    pending_substreams: HashSet<SubstreamId>,

    /// substream ID -> substream's inbound_tx channel
    substream_inbound_txs: HashMap<SubstreamId, UnboundedSender<Vec<u8>>>,

    /// substream ID -> substream's close_tx channel
    substream_close_txs: HashMap<SubstreamId, oneshot::Sender<()>>,

    /// send messages to the mixnet
    /// used for sending `SubstreamMessageType::OpenRequest` messages
    /// also passed to each substream so they can write to the mixnet
    pub(crate) mixnet_outbound_tx: UnboundedSender<OutboundMessage>,

    /// sender_tag for SURB replies to incoming messages
    pub(crate) sender_tag: Option<AnonymousSenderTag>,

    /// inbound substream open requests; used in poll_inbound
    inbound_open_tx: UnboundedSender<Substream>,
    inbound_open_rx: UnboundedReceiver<Substream>,

    /// closed substream IDs; used in poll_close
    close_tx: UnboundedSender<SubstreamId>,
    close_rx: UnboundedReceiver<SubstreamId>,

    /// message nonce contains the next nonce that should be used when
    /// sending a message over the connection
    pub(crate) message_nonce: Arc<AtomicU64>,

    waker: Option<Waker>,
}

impl Connection {
    pub(crate) fn new_with_sender_tag(
        peer_id: PeerId,
        remote_recipient: Option<Recipient>,
        id: ConnectionId,
        inbound_rx: UnboundedReceiver<SubstreamMessage>,
        mixnet_outbound_tx: UnboundedSender<OutboundMessage>,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Self {
        let (inbound_open_tx, inbound_open_rx) = unbounded();
        let (close_tx, close_rx) = unbounded();

        Connection {
            peer_id,
            remote_recipient,
            id,
            inbound_rx,
            pending_substreams: HashSet::new(),
            substream_inbound_txs: HashMap::new(),
            substream_close_txs: HashMap::new(),
            mixnet_outbound_tx,
            sender_tag,
            inbound_open_tx,
            inbound_open_rx,
            close_tx,
            close_rx,
            message_nonce: Arc::new(AtomicU64::new(1)),
            waker: None,
        }
    }

    /// Returns the remote peer's libp2p PeerId.
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Returns the remote peer's Nym address, if known.
    ///
    /// This is `Some(Recipient)` for connections we initiated (we know who we dialed),
    /// and `None` for incoming connections (we use SURBs to reply, never learning their address).
    ///
    /// **Privacy property**: For incoming connections, this is always `None` - the listener
    /// never learns the dialer's Nym address.
    pub fn remote_nym_address(&self) -> Option<Recipient> {
        self.remote_recipient
    }

    /// Returns true if this connection uses anonymous replies (SURBs).
    ///
    /// This is true for incoming connections where we don't know the remote's address.
    pub fn uses_anonymous_replies(&self) -> bool {
        self.sender_tag.is_some()
    }

    fn new_outbound_substream(&mut self) -> Result<Substream, Error> {
        debug!("new_outbound_substream called");
        let substream_id = SubstreamId::generate();
        debug!("Generated substream_id: {:?}", substream_id);
        let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);
        debug!("Using nonce {}", nonce);
        debug!("Connection sender_tag: {:?}", self.sender_tag);
        debug!(
            "About to send with sender_tag: {:?}",
            self.sender_tag.is_some()
        );

        let outbound_msg = OutboundMessage {
            recipient: self.remote_recipient, // Some(Recipient) for dialer, None for receiver
            message: Message::TransportMessage(TransportMessage {
                nonce,
                id: self.id.clone(),
                message: SubstreamMessage {
                    substream_id: substream_id.clone(),
                    message_type: SubstreamMessageType::OpenRequest,
                },
            }),
            sender_tag: self.sender_tag.clone(), // None for dialer, Some(sender_tag) for receiver
        };

        debug!("Sending OpenRequest for substream: {:?}", substream_id);
        // Send the outbound message
        self.mixnet_outbound_tx
            .unbounded_send(outbound_msg)
            .map_err(|e| {
                debug!("Failed to send outbound message: {}", e);
                Error::OutboundSendFailure(e.to_string())
            })?;

        debug!("Creating substream");
        // track pending outbound substreams
        let res = self.new_substream(substream_id.clone());
        if res.is_ok() {
            debug!("Adding to pending_substreams");
            self.pending_substreams.insert(substream_id);
        } else {
            debug!("Failed to create substream: {:?}", res);
        }
        res
    }

    // creates a new substream instance with the given ID.
    fn new_substream(&mut self, id: SubstreamId) -> Result<Substream, Error> {
        // check we don't already have a substream with this ID
        if self.substream_inbound_txs.contains_key(&id) {
            return Err(Error::SubstreamIdExists(id));
        }

        let (inbound_tx, inbound_rx) = unbounded::<Vec<u8>>();
        let (close_tx, close_rx) = oneshot::channel::<()>();
        self.substream_inbound_txs.insert(id.clone(), inbound_tx);
        self.substream_close_txs.insert(id.clone(), close_tx);

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        Ok(Substream::new_with_sender_tag(
            self.remote_recipient,
            self.id.clone(),
            id,
            inbound_rx,
            self.mixnet_outbound_tx.clone(),
            close_rx,
            self.message_nonce.clone(),
            self.sender_tag.clone(), // Pass the connection's SURB directly
        ))
    }

    fn handle_close(&mut self, substream_id: SubstreamId) -> Result<(), Error> {
        if self.substream_inbound_txs.remove(&substream_id).is_none() {
            return Err(Error::SubstreamIdDoesNotExist(substream_id));
        }

        // notify substream that it's closed
        let close_tx = self.substream_close_txs.remove(&substream_id);
        if let Some(tx) = close_tx {
            let _ = tx.send(());
        }

        // notify poll_close that the substream is closed
        self.close_tx
            .unbounded_send(substream_id)
            .map_err(|e| Error::InboundSendFailure(e.to_string()))
    }
}

impl StreamMuxer for Connection {
    type Substream = Substream;
    type Error = Error;

    fn poll_inbound(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Substream, Self::Error>> {
        console_log!("[Connection::poll_inbound] checking for inbound substreams");
        if let Poll::Ready(Some(substream)) = self.inbound_open_rx.poll_next_unpin(cx) {
            console_log!(
                "[Connection::poll_inbound] got inbound substream: {:?}",
                substream.substream_id
            );
            return Poll::Ready(Ok(substream));
        }

        Poll::Pending
    }

    fn poll_outbound(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Substream, Self::Error>> {
        console_log!("[Connection::poll_outbound] called");
        debug!("poll_outbound called");
        let result = self.new_outbound_substream();
        console_log!("[Connection::poll_outbound] result: {:?}", result.is_ok());
        debug!("poll_outbound result: {:?}", result.is_ok());
        Poll::Ready(result)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Poll::Ready(Some(_)) = self.close_rx.poll_next_unpin(cx) {
            return Poll::Ready(Ok(()));
        }

        Poll::Pending
    }

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<StreamMuxerEvent, Self::Error>> {
        while let Poll::Ready(Some(msg)) = self.inbound_rx.poll_next_unpin(cx) {
            debug!(
                "Connection poll received message type: {:?} for substream: {:?}",
                msg.message_type, msg.substream_id
            );
            match msg.message_type {
                SubstreamMessageType::OpenRequest => {
                    debug!(
                        "Processing OpenRequest for substream: {:?}",
                        msg.substream_id
                    );

                    if self.remote_recipient.is_none() {
                        debug!("Listener received OpenRequest - correct");
                    } else {
                        debug!("Dialer received OpenRequest - something is not right here");
                    }

                    // create a new substream with the given ID
                    let substream = self.new_substream(msg.substream_id.clone())?;
                    let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);

                    debug!("About to send OpenResponse with nonce: {}", nonce);
                    debug!("Using sender_tag: {:?}", self.sender_tag);

                    // send the response to the remote peer
                    let response_msg = OutboundMessage {
                        recipient: self.remote_recipient,
                        message: Message::TransportMessage(TransportMessage {
                            nonce,
                            id: self.id.clone(),
                            message: SubstreamMessage {
                                substream_id: msg.substream_id.clone(),
                                message_type: SubstreamMessageType::OpenResponse,
                            },
                        }),
                        sender_tag: self.sender_tag.clone(),
                    };

                    debug!("Created OutboundMessage: {:?}", response_msg);

                    self.mixnet_outbound_tx
                        .unbounded_send(response_msg)
                        .map_err(|e| {
                            debug!("FAILED to send OpenResponse: {}", e);
                            Error::OutboundSendFailure(e.to_string())
                        })?;
                    debug!("Queued OpenResponse for mixnet");

                    // send the substream to our own channel to be returned in poll_inbound
                    self.inbound_open_tx
                        .unbounded_send(substream)
                        .map_err(|e| Error::InboundSendFailure(e.to_string()))?;

                    debug!("new inbound substream: {:?}", &msg.substream_id);
                }
                SubstreamMessageType::OpenResponse => {
                    debug!(
                        "Processing OpenResponse for substream: {:?}",
                        msg.substream_id
                    );
                    if !self.pending_substreams.remove(&msg.substream_id) {
                        debug!(
                            "SubstreamMessageType::OpenResponse no substream pending for ID: {:?}",
                            &msg.substream_id
                        );
                    }
                }
                SubstreamMessageType::Close => {
                    debug!("Processing Close for substream: {:?}", msg.substream_id);
                    self.handle_close(msg.substream_id)?;
                }
                SubstreamMessageType::Data(data) => {
                    console_log!(
                        "[Connection::poll] Data received: {} bytes for substream {:?}",
                        data.len(),
                        msg.substream_id
                    );
                    debug!("Processing Data: {:?}", &data);
                    let inbound_tx = self.substream_inbound_txs.get_mut(&msg.substream_id);

                    match inbound_tx {
                        Some(tx) => {
                            console_log!("[Connection::poll] Forwarding data to substream channel");
                            if let Err(e) = tx.unbounded_send(data) {
                                console_log!("[Connection::poll] ERROR: Channel closed for substream {:?}: {}", msg.substream_id, e);
                            }
                        }
                        None => {
                            console_log!("[Connection::poll] WARNING: No channel for substream {:?}, dropping data", msg.substream_id);
                        }
                    }
                }
            }
        }

        self.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// PendingConnection represents a connection that's been initiated, but not completed.
pub(crate) struct PendingConnection {
    pub(crate) remote_recipient: Recipient,
    pub(crate) connection_tx: oneshot::Sender<Connection>,
}

impl PendingConnection {
    pub(crate) fn new(
        remote_recipient: Recipient,
        connection_tx: oneshot::Sender<Connection>,
    ) -> Self {
        PendingConnection {
            remote_recipient,
            connection_tx,
        }
    }
}
