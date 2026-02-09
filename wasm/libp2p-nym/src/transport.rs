// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! NymTransport implementation of libp2p Transport trait for WASM.

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use futures::prelude::*;
use gloo_timers::future::TimeoutFuture;
use libp2p::core::{
    multiaddr::Multiaddr,
    transport::{DialOpts, ListenerId, TransportError, TransportEvent},
    Transport,
};
use libp2p_identity::{Keypair, PeerId};
use log::{debug, info};
use nym_client_wasm::stream::NymClientStream;
use nym_sphinx_addressing::clients::Recipient;
use send_wrapper::SendWrapper;
use std::{
    collections::HashMap,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll, Waker},
    time::Duration,
};

use super::connection::{Connection, PendingConnection};
use super::error::Error;
use super::message::{
    ConnectionId, ConnectionMessage, InboundMessage, Message, OutboundMessage, SubstreamMessage,
    TransportMessage,
};
use super::mixnet::initialize_mixnet;
use super::queue::MessageQueue;
use super::DEFAULT_HANDSHAKE_TIMEOUT_SECS;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;

/// InboundTransportEvent represents an inbound event from the mixnet.
pub enum InboundTransportEvent {
    ConnectionRequest(Upgrade),
    ConnectionResponse,
    TransportMessage,
}

/// NymTransport implements the Transport trait using the Nym mixnet.
pub struct NymTransport {
    /// our Nym address
    self_address: Recipient,
    pub(crate) listen_addr: Multiaddr,
    pub(crate) listener_id: ListenerId,

    /// our libp2p keypair; currently not really used
    keypair: Keypair,

    /// established connections -> channel which sends messages received from
    /// the mixnet to the corresponding Connection
    connections: HashMap<ConnectionId, UnboundedSender<SubstreamMessage>>,

    /// outbound pending dials
    pending_dials: HashMap<ConnectionId, PendingConnection>,

    /// connection message queues
    message_queues: HashMap<ConnectionId, MessageQueue>,

    /// inbound mixnet messages
    inbound_rx: UnboundedReceiver<InboundMessage>,

    /// outbound mixnet messages
    outbound_tx: UnboundedSender<OutboundMessage>,

    /// inbound messages for Transport.poll()
    poll_rx: UnboundedReceiver<TransportEvent<Upgrade, Error>>,

    /// outbound messages to Transport.poll()
    poll_tx: UnboundedSender<TransportEvent<Upgrade, Error>>,

    waker: Option<Waker>,

    /// Timeout for the [`Upgrade`] future (in milliseconds for WASM).
    handshake_timeout_ms: u32,
}

impl NymTransport {
    /// Create a new NymTransport from a NymClientStream.
    ///
    /// # Example
    /// ```ignore
    /// use nym_libp2p_wasm::{create_transport_client_async, NymTransport};
    /// use libp2p_identity::Keypair;
    ///
    /// // Create the transport client
    /// let result = create_transport_client_async(None).await?;
    ///
    /// // Create the transport
    /// let keypair = Keypair::generate_ed25519();
    /// let transport = NymTransport::new(
    ///     result.self_address,
    ///     result.stream,
    ///     keypair,
    /// ).await?;
    /// ```
    pub async fn new(
        self_address: Recipient,
        stream: NymClientStream,
        keypair: Keypair,
    ) -> Result<Self, Error> {
        Self::new_with_options(self_address, stream, keypair, None, None).await
    }

    /// Create a new NymTransport with a custom timeout.
    pub async fn new_with_timeout(
        self_address: Recipient,
        stream: NymClientStream,
        keypair: Keypair,
        timeout: Duration,
    ) -> Result<Self, Error> {
        Self::new_with_options(self_address, stream, keypair, None, Some(timeout)).await
    }

    /// Add timeout to transport and return self.
    #[allow(dead_code)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.handshake_timeout_ms = timeout.as_millis() as u32;
        self
    }

    async fn new_with_options(
        self_address: Recipient,
        stream: NymClientStream,
        keypair: Keypair,
        notify_inbound_tx: Option<UnboundedSender<()>>,
        timeout: Option<Duration>,
    ) -> Result<Self, Error> {
        let (self_address, inbound_rx, outbound_tx) =
            initialize_mixnet(self_address, stream, notify_inbound_tx).await?;
        let listen_addr = nym_address_to_multiaddress(self_address)?;
        let listener_id = ListenerId::next();

        let (poll_tx, poll_rx) = unbounded::<TransportEvent<Upgrade, Error>>();

        poll_tx
            .unbounded_send(TransportEvent::NewAddress {
                listener_id,
                listen_addr: listen_addr.clone(),
            })
            .map_err(|_| Error::SendErrorTransportEvent)?;

        let handshake_timeout_ms = timeout
            .map(|t| t.as_millis() as u32)
            .unwrap_or((DEFAULT_HANDSHAKE_TIMEOUT_SECS * 1000) as u32);

        Ok(Self {
            self_address,
            listen_addr,
            listener_id,
            keypair,
            connections: HashMap::new(),
            pending_dials: HashMap::new(),
            message_queues: HashMap::new(),
            inbound_rx,
            outbound_tx,
            poll_rx,
            poll_tx,
            waker: None,
            handshake_timeout_ms,
        })
    }

    pub(crate) fn peer_id(&self) -> PeerId {
        PeerId::from_public_key(&self.keypair.public())
    }

    fn handle_message_queue_on_connection_initiation(
        &mut self,
        id: &ConnectionId,
    ) -> Result<(), Error> {
        debug!("handle_message_queue_on_connection_initiation");
        let Some(inbound_tx) = self.connections.get(id) else {
            // this should not happen
            return Err(Error::NoConnectionForTransportMessage);
        };

        match self.message_queues.get_mut(id) {
            Some(queue) => {
                // update expected nonce
                queue.set_connection_message_received();

                // push pending inbound some messages in this case
                while let Some(msg) = queue.pop() {
                    debug!(
                        "popped queued message with nonce {} for connection",
                        msg.nonce
                    );
                    inbound_tx
                        .unbounded_send(msg.message.clone())
                        .map_err(|e| Error::InboundSendFailure(e.to_string()))?;
                }
            }
            None => {
                // no queue exists for this connection, create one
                let queue = MessageQueue::new();
                self.message_queues.insert(id.clone(), queue);
                let queue = self.message_queues.get_mut(id).unwrap();
                queue.set_connection_message_received();
            }
        };

        debug!("returning from handle_message_queue_on_connection_initiation");
        Ok(())
    }

    // handle_connection_response resolves the pending connection corresponding to the response
    // (if there is one) into a Connection.
    fn handle_connection_response(
        &mut self,
        msg: &ConnectionMessage,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Result<(), Error> {
        if self.connections.contains_key(&msg.id) {
            return Err(Error::ConnectionAlreadyEstablished);
        }

        if let Some(pending_conn) = self.pending_dials.remove(&msg.id) {
            // Create connection with sender_tag
            let (conn, conn_tx) = self.create_connection_types(
                msg.peer_id,
                Some(pending_conn.remote_recipient), // Dialer knows recipient
                msg.id.clone(),
                sender_tag,
            );

            self.connections.insert(msg.id.clone(), conn_tx);
            self.handle_message_queue_on_connection_initiation(&msg.id)?;

            pending_conn
                .connection_tx
                .send(conn)
                .map_err(|_| Error::ConnectionSendFailure)?;

            if let Some(waker) = self.waker.take() {
                waker.wake();
            }

            Ok(())
        } else {
            Err(Error::NoConnectionForResponse)
        }
    }

    /// handle_connection_request handles an incoming connection request, sends back a
    /// connection response, and finally completes the upgrade into a Connection.
    fn handle_connection_request(
        &mut self,
        msg: &ConnectionMessage,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Result<Connection, Error> {
        // ensure we don't already have a conn with the same id
        if self.connections.contains_key(&msg.id) {
            return Err(Error::ConnectionIDExists);
        }

        // Create connection with sender_tag
        let (conn, conn_tx) = self.create_connection_types(
            msg.peer_id,
            None, // Receiver doesn't know dialer address
            msg.id.clone(),
            sender_tag.clone(),
        );

        info!("Created connection: {:?}", conn);

        self.connections.insert(msg.id.clone(), conn_tx);
        info!("Current active connections: {}", self.connections.len());

        self.handle_message_queue_on_connection_initiation(&msg.id)?;

        let resp = ConnectionMessage {
            peer_id: self.peer_id(),
            id: msg.id.clone(),
        };

        // Send response using sender_tag if available
        self.outbound_tx
            .unbounded_send(OutboundMessage {
                message: Message::ConnectionResponse(resp),
                recipient: None,
                sender_tag,
            })
            .map_err(|e| Error::OutboundSendFailure(e.to_string()))?;

        debug!(
            "Sent ConnectionResponse with sender_tag: {:?}",
            sender_tag.is_some()
        );
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        Ok(conn)
    }

    fn handle_transport_message(&mut self, msg: TransportMessage) -> Result<(), Error> {
        let queue = match self.message_queues.get_mut(&msg.id) {
            Some(queue) => queue,
            None => {
                // no queue exists for this connection, create one
                let queue = MessageQueue::new();
                self.message_queues.insert(msg.id.clone(), queue);
                self.message_queues.get_mut(&msg.id).unwrap()
            }
        };

        queue.print_nonces();

        let nonce = msg.nonce;
        let Some(msg) = queue.try_push(msg) else {
            // don't push the message yet, it's been queued
            debug!("message with nonce {} queued for connection", nonce);
            return Ok(());
        };

        let Some(inbound_tx) = self.connections.get(&msg.id) else {
            return Err(Error::NoConnectionForTransportMessage);
        };

        // send original message
        debug!(
            "sending original message with nonce {} for connection",
            nonce
        );
        inbound_tx
            .unbounded_send(msg.message.clone())
            .map_err(|e| Error::InboundSendFailure(e.to_string()))?;

        // try to pop queued messages and send them on inbound channel
        while let Some(msg) = queue.pop() {
            debug!(
                "popped queued message with nonce {} for connection",
                msg.nonce
            );
            inbound_tx
                .unbounded_send(msg.message.clone())
                .map_err(|e| Error::InboundSendFailure(e.to_string()))?;
        }

        if let Some(waker) = self.waker.clone().take() {
            waker.wake();
        }

        Ok(())
    }

    fn create_connection_types(
        &self,
        remote_peer_id: PeerId,
        remote_recipient: Option<Recipient>,
        id: ConnectionId,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> (Connection, UnboundedSender<SubstreamMessage>) {
        let (inbound_tx, inbound_rx) = unbounded::<SubstreamMessage>();

        let conn = Connection::new_with_sender_tag(
            remote_peer_id,
            remote_recipient,
            id,
            inbound_rx,
            self.outbound_tx.clone(),
            sender_tag,
        );

        (conn, inbound_tx)
    }

    /// handle_inbound handles an inbound message from the mixnet, received via self.inbound_rx.
    fn handle_inbound(
        &mut self,
        msg: Message,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Result<InboundTransportEvent, Error> {
        match msg {
            Message::ConnectionRequest(inner) => {
                debug!("got inbound connection request {:?}", inner);
                match self.handle_connection_request(&inner, sender_tag) {
                    Ok(conn) => {
                        let (connection_tx, connection_rx) =
                            oneshot::channel::<(PeerId, Connection)>();
                        let upgrade = Upgrade::new(connection_rx);
                        connection_tx
                            .send((inner.peer_id, conn))
                            .map_err(|_| Error::ConnectionSendFailure)?;
                        Ok(InboundTransportEvent::ConnectionRequest(upgrade))
                    }
                    Err(e) => Err(e),
                }
            }
            Message::ConnectionResponse(msg) => {
                debug!("got inbound connection response {:?}", msg);
                self.handle_connection_response(&msg, sender_tag)
                    .map(|_| InboundTransportEvent::ConnectionResponse)
            }
            Message::TransportMessage(msg) => {
                debug!(
                    "Transport received TransportMessage: nonce={}, substream={:?}, msg_type={:?}",
                    msg.nonce, msg.message.substream_id, msg.message.message_type
                );
                self.handle_transport_message(msg)
                    .map(|_| InboundTransportEvent::TransportMessage)
            }
        }
    }
}

/// Upgrade represents a transport listener upgrade.
/// Note: we immediately upgrade a connection request to a connection,
/// so this only contains a channel for receiving that connection.
pub struct Upgrade {
    connection_rx: oneshot::Receiver<(PeerId, Connection)>,
}

impl Upgrade {
    fn new(connection_rx: oneshot::Receiver<(PeerId, Connection)>) -> Upgrade {
        Upgrade { connection_rx }
    }
}

impl Future for Upgrade {
    type Output = Result<(PeerId, Connection), Error>;

    // poll checks if the upgrade has turned into a connection yet
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.connection_rx.poll_unpin(cx) {
            Poll::Ready(Ok(conn)) => Poll::Ready(Ok(conn)),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::RecvFailure)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Transport for NymTransport {
    type Output = (PeerId, Connection);
    type Error = Error;
    type ListenerUpgrade = Upgrade;
    // Use SendWrapper to make the future Send for libp2p's SwarmBuilder
    // This is safe in WASM's single-threaded environment
    type Dial = futures::future::BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn listen_on(
        &mut self,
        _listener_id: ListenerId,
        _multi_addr: libp2p::Multiaddr,
    ) -> Result<(), TransportError<Self::Error>> {
        info!("called listen_on, this is currently just a dummy function - client starts listening on new()");
        Ok(())
    }

    fn remove_listener(&mut self, id: ListenerId) -> bool {
        if self.listener_id != id {
            return false;
        }

        let _ = self.poll_tx.unbounded_send(TransportEvent::ListenerClosed {
            listener_id: id,
            reason: Ok(()),
        });
        true
    }

    fn dial(
        &mut self,
        addr: Multiaddr,
        _dial_opts: DialOpts,
    ) -> Result<Self::Dial, TransportError<Self::Error>> {
        debug!("dialing {}", addr);

        let id = ConnectionId::generate();

        // create remote recipient address
        let recipient = multiaddress_to_nym_address(addr).map_err(TransportError::Other)?;

        // create pending conn structs and store
        let (connection_tx, connection_rx) = oneshot::channel::<Connection>();

        let inner_pending_conn = PendingConnection::new(recipient, connection_tx);
        self.pending_dials.insert(id.clone(), inner_pending_conn);

        let local_key = Keypair::generate_ed25519();
        let connection_peer_id = PeerId::from(local_key.public());

        // put ConnectionRequest message into outbound message channel
        let msg = ConnectionMessage {
            peer_id: connection_peer_id,
            id,
        };

        let outbound_tx = self.outbound_tx.clone();

        let mut waker = self.waker.clone();
        let handshake_timeout_ms = self.handshake_timeout_ms;

        // Wrap in SendWrapper to satisfy Send bounds for SwarmBuilder
        // This is safe because WASM is single-threaded
        Ok(SendWrapper::new(async move {
            outbound_tx
                .unbounded_send(OutboundMessage {
                    message: Message::ConnectionRequest(msg),
                    recipient: Some(recipient),
                    sender_tag: None,
                })
                .map_err(|e| Error::OutboundSendFailure(e.to_string()))?;

            debug!("sent outbound ConnectionRequest");
            if let Some(waker) = waker.take() {
                waker.wake();
            };

            // Use gloo_timers for WASM-compatible timeout
            let timeout_future = TimeoutFuture::new(handshake_timeout_ms);

            futures::select! {
                conn_result = connection_rx.fuse() => {
                    match conn_result {
                        Ok(conn) => Ok((conn.peer_id, conn)),
                        Err(_) => Err(Error::RecvFailure),
                    }
                }
                _ = timeout_future.fuse() => {
                    Err(Error::DialTimeout)
                }
            }
        })
        .boxed())
    }

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<TransportEvent<Self::ListenerUpgrade, Self::Error>> {
        // new addresses + listener close events
        if let Poll::Ready(Some(res)) = self.poll_rx.poll_next_unpin(cx) {
            return Poll::Ready(res);
        }

        // check for and handle inbound messages
        while let Poll::Ready(Some(msg)) = self.inbound_rx.poll_next_unpin(cx) {
            debug!(
                "TRANSPORT: Received inbound message type: {:?}",
                match &msg.0 {
                    Message::ConnectionRequest(_) => "ConnectionRequest",
                    Message::ConnectionResponse(_) => "ConnectionResponse",
                    Message::TransportMessage(_) => "TransportMessage",
                }
            );

            match self.handle_inbound(msg.0, msg.1) {
                Ok(event) => match event {
                    InboundTransportEvent::ConnectionRequest(upgrade) => {
                        info!("InboundTransportEvent::ConnectionRequest");
                        return Poll::Ready(TransportEvent::Incoming {
                            listener_id: self.listener_id,
                            upgrade,
                            local_addr: self.listen_addr.clone(),
                            send_back_addr: self.listen_addr.clone(),
                        });
                    }
                    InboundTransportEvent::ConnectionResponse => {
                        info!("InboundTransportEvent::ConnectionResponse");
                    }
                    InboundTransportEvent::TransportMessage => {
                        debug!("InboundTransportEvent::TransportMessage");
                    }
                },
                Err(e) => {
                    return Poll::Ready(TransportEvent::ListenerError {
                        listener_id: self.listener_id,
                        error: e,
                    });
                }
            };
        }

        self.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// Convert a Nym Recipient address to a libp2p Multiaddr.
///
/// Format: `/nym/<base58-encoded-nym-address>`
pub fn nym_address_to_multiaddress(addr: Recipient) -> Result<Multiaddr, Error> {
    // Create a multiaddr using the Nym protocol
    // Format: /nym/<base58-encoded-nym-address>
    // This requires the ChainSafe multiaddr fork with Protocol::Nym support
    Multiaddr::from_str(&format!("/nym/{}", addr)).map_err(Error::FailedToFormatMultiaddr)
}

fn multiaddress_to_nym_address(multiaddr: Multiaddr) -> Result<Recipient, Error> {
    // Parse the Nym address from the multiaddr
    // We expect format: /nym/<nym-address>
    let addr_str = multiaddr.to_string();

    if let Some(nym_addr) = addr_str.strip_prefix("/nym/") {
        return Recipient::from_str(nym_addr).map_err(Error::InvalidRecipientBytes);
    }

    Err(Error::InvalidProtocolForMultiaddr)
}
