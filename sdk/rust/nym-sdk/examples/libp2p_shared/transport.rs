use futures::prelude::*;
use libp2p::core::{
    identity::Keypair,
    multiaddr::{Multiaddr, Protocol},
    transport::{ListenerId, TransportError, TransportEvent},
    PeerId, Transport,
};
use log::debug;
use nym_sdk::mixnet::MixnetClient;
use nym_sphinx::addressing::clients::Recipient;
use std::{
    collections::HashMap,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll, Waker},
};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    time::{timeout, Duration},
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::connection::{Connection, PendingConnection};
use super::error::Error;
use super::message::{
    ConnectionId, ConnectionMessage, InboundMessage, Message, OutboundMessage, SubstreamMessage,
    TransportMessage,
};
use super::mixnet::initialize_mixnet;
use super::queue::MessageQueue;
use super::DEFAULT_HANDSHAKE_TIMEOUT_SECS;

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
    inbound_stream: UnboundedReceiverStream<InboundMessage>,

    /// outbound mixnet messages
    outbound_tx: UnboundedSender<OutboundMessage>,

    /// inbound messages for Transport.poll()
    poll_rx: UnboundedReceiver<TransportEvent<Upgrade, Error>>,

    /// outbound messages to Transport.poll()
    poll_tx: UnboundedSender<TransportEvent<Upgrade, Error>>,

    waker: Option<Waker>,

    /// Timeout for the [`Upgrade`] future.
    handshake_timeout: Duration,
}

impl NymTransport {
    /// New transport.
    #[allow(unused)]
    pub async fn new(client: MixnetClient, keypair: Keypair) -> Result<Self, Error> {
        Self::new_maybe_with_notify_inbound(client, keypair, None, None).await
    }

    /// New transport with a timeout.
    #[allow(dead_code)]
    pub async fn new_with_timeout(
        client: MixnetClient,
        keypair: Keypair,
        timeout: Duration,
    ) -> Result<Self, Error> {
        Self::new_maybe_with_notify_inbound(client, keypair, None, Some(timeout)).await
    }

    /// Add timeout to transport and return self.
    #[allow(dead_code)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.handshake_timeout = timeout;
        self
    }

    async fn new_maybe_with_notify_inbound(
        client: MixnetClient,
        keypair: Keypair,
        notify_inbound_tx: Option<UnboundedSender<()>>,
        timeout: Option<Duration>,
    ) -> Result<Self, Error> {
        let (self_address, inbound_rx, outbound_tx) =
            initialize_mixnet(client, notify_inbound_tx).await?;
        let listen_addr = nym_address_to_multiaddress(self_address)?;
        let listener_id = ListenerId::new();

        let (poll_tx, poll_rx) = unbounded_channel::<TransportEvent<Upgrade, Error>>();

        poll_tx
            .send(TransportEvent::NewAddress {
                listener_id,
                listen_addr: listen_addr.clone(),
            })
            .map_err(|_| Error::SendErrorTransportEvent)?;

        let inbound_stream = UnboundedReceiverStream::new(inbound_rx);
        let handshake_timeout =
            timeout.unwrap_or_else(|| Duration::from_secs(DEFAULT_HANDSHAKE_TIMEOUT_SECS));

        Ok(Self {
            self_address,
            listen_addr,
            listener_id,
            keypair,
            connections: HashMap::new(),
            pending_dials: HashMap::new(),
            message_queues: HashMap::new(),
            inbound_stream,
            outbound_tx,
            poll_rx,
            poll_tx,
            waker: None,
            handshake_timeout,
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
                        .send(msg.message.clone())
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
    fn handle_connection_response(&mut self, msg: &ConnectionMessage) -> Result<(), Error> {
        if self.connections.contains_key(&msg.id) {
            return Err(Error::ConnectionAlreadyEstablished);
        }

        if let Some(pending_conn) = self.pending_dials.remove(&msg.id) {
            // resolve connection and put into pending_conn channel
            let (conn, conn_tx) = self.create_connection_types(
                msg.peer_id,
                pending_conn.remote_recipient,
                msg.id.clone(),
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
    fn handle_connection_request(&mut self, msg: &ConnectionMessage) -> Result<Connection, Error> {
        if msg.recipient.is_none() {
            return Err(Error::NoneRecipientInConnectionRequest);
        }

        // ensure we don't already have a conn with the same id
        if self.connections.contains_key(&msg.id) {
            return Err(Error::ConnectionIDExists);
        }

        let (conn, conn_tx) =
            self.create_connection_types(msg.peer_id, msg.recipient.unwrap(), msg.id.clone());
        self.connections.insert(msg.id.clone(), conn_tx);
        self.handle_message_queue_on_connection_initiation(&msg.id)?;

        let resp = ConnectionMessage {
            peer_id: self.peer_id(),
            recipient: None,
            id: msg.id.clone(),
        };

        self.outbound_tx
            .send(OutboundMessage {
                message: Message::ConnectionResponse(resp),
                recipient: msg.recipient.unwrap(),
            })
            .map_err(|e| Error::OutboundSendFailure(e.to_string()))?;

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
            .send(msg.message.clone())
            .map_err(|e| Error::InboundSendFailure(e.to_string()))?;

        // try to pop queued messages and send them on inbound channel
        while let Some(msg) = queue.pop() {
            debug!(
                "popped queued message with nonce {} for connection",
                msg.nonce
            );
            inbound_tx
                .send(msg.message.clone())
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
        recipient: Recipient,
        id: ConnectionId,
    ) -> (Connection, UnboundedSender<SubstreamMessage>) {
        let (inbound_tx, inbound_rx) = unbounded_channel::<SubstreamMessage>();

        // representation of a connection; this contains channels for applications to read/write to.
        let conn = Connection::new(
            remote_peer_id,
            recipient,
            id,
            inbound_rx,
            self.outbound_tx.clone(),
        );

        // inbound_tx is what we write to when receiving messages on the mixnet,
        (conn, inbound_tx)
    }

    /// handle_inbound handles an inbound message from the mixnet, received via self.inbound_stream.
    fn handle_inbound(&mut self, msg: Message) -> Result<InboundTransportEvent, Error> {
        match msg {
            Message::ConnectionRequest(inner) => {
                debug!("got inbound connection request {:?}", inner);
                match self.handle_connection_request(&inner) {
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
                self.handle_connection_response(&msg)
                    .map(|_| InboundTransportEvent::ConnectionResponse)
            }
            Message::TransportMessage(msg) => {
                debug!("got inbound TransportMessage: {:?}", msg);
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
    connection_tx: oneshot::Receiver<(PeerId, Connection)>,
}

impl Upgrade {
    fn new(connection_tx: oneshot::Receiver<(PeerId, Connection)>) -> Upgrade {
        Upgrade { connection_tx }
    }
}

impl Future for Upgrade {
    type Output = Result<(PeerId, Connection), Error>;

    // poll checks if the upgrade has turned into a connection yet
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.connection_tx
            .poll_unpin(cx)
            .map_err(|_| Error::RecvFailure)
    }
}

impl Transport for NymTransport {
    type Output = (PeerId, Connection);
    type Error = Error;
    type ListenerUpgrade = Upgrade;
    type Dial = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn listen_on(&mut self, _: Multiaddr) -> Result<ListenerId, TransportError<Self::Error>> {
        // we should only allow listening on the multiaddress containing our Nym address
        Ok(self.listener_id)
    }

    fn remove_listener(&mut self, id: ListenerId) -> bool {
        if self.listener_id != id {
            return false;
        }

        // TODO: close channels?
        self.poll_tx
            .send(TransportEvent::ListenerClosed {
                listener_id: id,
                reason: Ok(()),
            })
            .expect("failed to send listener closed event");
        true
    }

    fn dial(&mut self, addr: Multiaddr) -> Result<Self::Dial, TransportError<Self::Error>> {
        debug!("dialing {}", addr);

        let id = ConnectionId::generate();

        // create remote recipient address
        let recipient = multiaddress_to_nym_address(addr).map_err(TransportError::Other)?;

        // create pending conn structs and store
        let (connection_tx, connection_rx) = oneshot::channel::<Connection>();

        let inner_pending_conn = PendingConnection::new(recipient, connection_tx);
        self.pending_dials.insert(id.clone(), inner_pending_conn);

        // put ConnectionRequest message into outbound message channel
        let msg = ConnectionMessage {
            peer_id: self.peer_id(),
            recipient: Some(self.self_address),
            id,
        };

        let outbound_tx = self.outbound_tx.clone();

        let mut waker = self.waker.clone();
        let handshake_timeout = self.handshake_timeout;
        Ok(async move {
            outbound_tx
                .send(OutboundMessage {
                    message: Message::ConnectionRequest(msg),
                    recipient,
                })
                .map_err(|e| Error::OutboundSendFailure(e.to_string()))?;

            debug!("sent outbound ConnectionRequest");
            if let Some(waker) = waker.take() {
                waker.wake();
            };

            let conn = timeout(handshake_timeout, connection_rx).await??;
            Ok((conn.peer_id, conn))
        }
        .boxed())
    }

    // dial_as_listener currently just calls self.dial().
    fn dial_as_listener(
        &mut self,
        addr: Multiaddr,
    ) -> Result<Self::Dial, TransportError<Self::Error>> {
        self.dial(addr)
    }

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<TransportEvent<Self::ListenerUpgrade, Self::Error>> {
        // new addresses + listener close events
        if let Poll::Ready(Some(res)) = self.poll_rx.recv().boxed().poll_unpin(cx) {
            return Poll::Ready(res);
        }

        // check for and handle inbound messages
        while let Poll::Ready(Some(msg)) = self.inbound_stream.poll_next_unpin(cx) {
            match self.handle_inbound(msg.0) {
                Ok(event) => match event {
                    InboundTransportEvent::ConnectionRequest(upgrade) => {
                        debug!("InboundTransportEvent::ConnectionRequest");
                        return Poll::Ready(TransportEvent::Incoming {
                            listener_id: self.listener_id,
                            upgrade,
                            local_addr: self.listen_addr.clone(),
                            send_back_addr: self.listen_addr.clone(),
                        });
                    }
                    InboundTransportEvent::ConnectionResponse => {
                        debug!("InboundTransportEvent::ConnectionResponse");
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

    fn address_translation(&self, _listen: &Multiaddr, _observed: &Multiaddr) -> Option<Multiaddr> {
        None
    }
}

fn nym_address_to_multiaddress(addr: Recipient) -> Result<Multiaddr, Error> {
    Multiaddr::from_str(&format!("/nym/{}", addr)).map_err(Error::FailedToFormatMultiaddr)
}

fn multiaddress_to_nym_address(multiaddr: Multiaddr) -> Result<Recipient, Error> {
    let mut multiaddr = multiaddr;
    match multiaddr.pop().unwrap() {
        Protocol::Nym(addr) => Recipient::from_str(&addr).map_err(Error::InvalidRecipientBytes),
        _ => Err(Error::InvalidProtocolForMultiaddr),
    }
}

#[cfg(test)]
mod test {
    use super::super::connection::Connection;
    use super::super::error::Error;
    use super::super::message::{
        Message, OutboundMessage, SubstreamId, SubstreamMessage, SubstreamMessageType,
        TransportMessage,
    };
    use super::super::substream::Substream;
    use super::{nym_address_to_multiaddress, NymTransport};
    use futures::{future::poll_fn, AsyncReadExt, AsyncWriteExt, FutureExt};
    use libp2p::core::{
        identity::Keypair,
        transport::{Transport, TransportEvent},
        Multiaddr, StreamMuxer,
    };
    use log::info;
    use nym_bin_common::logging::setup_logging;
    use nym_sdk::mixnet::MixnetClient;
    use std::{pin::Pin, str::FromStr, sync::atomic::Ordering};
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

    impl Connection {
        fn write(&self, msg: SubstreamMessage) -> Result<(), Error> {
            let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);
            self.mixnet_outbound_tx
                .send(OutboundMessage {
                    recipient: self.remote_recipient,
                    message: Message::TransportMessage(TransportMessage {
                        nonce,
                        id: self.id.clone(),
                        message: msg,
                    }),
                })
                .map_err(|e| Error::OutboundSendFailure(e.to_string()))?;
            Ok(())
        }
    }

    impl NymTransport {
        async fn new_with_notify_inbound(
            client: MixnetClient,
            notify_inbound_tx: UnboundedSender<()>,
        ) -> Result<Self, Error> {
            let local_key = Keypair::generate_ed25519();
            Self::new_maybe_with_notify_inbound(client, local_key, Some(notify_inbound_tx), None)
                .await
        }
    }

    #[tokio::test]
    async fn test_transport_connection() {
        setup_logging();

        let client = MixnetClient::connect_new().await.unwrap();
        let (dialer_notify_inbound_tx, mut dialer_notify_inbound_rx) = unbounded_channel();
        let mut dialer_transport =
            NymTransport::new_with_notify_inbound(client, dialer_notify_inbound_tx)
                .await
                .unwrap();

        let client2 = MixnetClient::connect_new().await.unwrap();
        let (listener_notify_inbound_tx, mut listener_notify_inbound_rx) = unbounded_channel();
        let mut listener_transport =
            NymTransport::new_with_notify_inbound(client2, listener_notify_inbound_tx)
                .await
                .unwrap();
        let listener_multiaddr =
            nym_address_to_multiaddress(listener_transport.self_address).unwrap();
        assert_new_address_event(Pin::new(&mut dialer_transport)).await;
        assert_new_address_event(Pin::new(&mut listener_transport)).await;

        // dial the remote peer
        let mut dial = dialer_transport.dial(listener_multiaddr).unwrap();

        // poll the dial to send the connection request message
        assert!(poll_fn(|cx| Pin::new(&mut dial).as_mut().poll_unpin(cx))
            .now_or_never()
            .is_none());
        listener_notify_inbound_rx.recv().await.unwrap();

        // should receive the connection request from the mixnet and send the connection response
        let res = poll_fn(|cx| Pin::new(&mut listener_transport).as_mut().poll(cx)).await;
        let mut upgrade = match res {
            TransportEvent::Incoming {
                listener_id,
                upgrade,
                local_addr,
                send_back_addr,
            } => {
                assert_eq!(listener_id, listener_transport.listener_id);
                assert_eq!(local_addr, listener_transport.listen_addr);
                assert_eq!(send_back_addr, listener_transport.listen_addr);
                upgrade
            }
            _ => panic!("expected TransportEvent::Incoming, got {:?}", res),
        };
        dialer_notify_inbound_rx.recv().await.unwrap();

        // should receive the connection response from the mixnet
        assert!(
            poll_fn(|cx| Pin::new(&mut dialer_transport).as_mut().poll(cx))
                .now_or_never()
                .is_none()
        );
        info!("waiting for connections...");

        // should be able to resolve the connections now
        let (_, mut listener_conn) = poll_fn(|cx| Pin::new(&mut upgrade).as_mut().poll_unpin(cx))
            .now_or_never()
            .expect("the upgrade should be ready")
            .expect("the upgrade should not error");
        let (_, mut dialer_conn) = poll_fn(|cx| Pin::new(&mut dial).as_mut().poll_unpin(cx))
            .now_or_never()
            .expect("the upgrade should be ready")
            .expect("the upgrade should not error");
        info!("connections established");

        // write messages from the dialer to the listener and vice versa
        send_and_receive_over_conns(
            b"hello".to_vec(),
            &mut dialer_conn,
            &mut listener_conn,
            Pin::new(&mut listener_transport),
            &mut listener_notify_inbound_rx,
        )
        .await;
        send_and_receive_over_conns(
            b"hi".to_vec(),
            &mut dialer_conn,
            &mut listener_conn,
            Pin::new(&mut listener_transport),
            &mut listener_notify_inbound_rx,
        )
        .await;
        send_and_receive_over_conns(
            b"world".to_vec(),
            &mut listener_conn,
            &mut dialer_conn,
            Pin::new(&mut dialer_transport),
            &mut dialer_notify_inbound_rx,
        )
        .await;
    }

    async fn assert_new_address_event(mut transport: Pin<&mut NymTransport>) {
        match poll_fn(|cx| transport.as_mut().poll(cx)).await {
            TransportEvent::NewAddress {
                listener_id,
                listen_addr,
            } => {
                assert_eq!(listener_id, transport.listener_id);
                assert_eq!(listen_addr, transport.listen_addr);
            }
            _ => panic!("expected TransportEvent::NewAddress"),
        }
    }

    async fn send_and_receive_over_conns(
        msg: Vec<u8>,
        conn1: &mut Connection,
        conn2: &mut Connection,
        mut transport2: Pin<&mut NymTransport>,
        notify_inbound_rx: &mut UnboundedReceiver<()>,
    ) {
        // send message over conn1 to conn2
        let substream_id = SubstreamId::generate();
        conn1
            .write(SubstreamMessage::new_with_data(
                substream_id.clone(),
                msg.clone(),
            ))
            .unwrap();
        notify_inbound_rx.recv().await.unwrap();

        // poll transport2 to push message from transport to connection
        assert!(poll_fn(|cx| transport2.as_mut().poll(cx))
            .now_or_never()
            .is_none());
        let substream_msg = conn2.inbound_rx.recv().await.unwrap();
        if let SubstreamMessageType::Data(data) = substream_msg.message_type {
            assert_eq!(data, msg);
        } else {
            panic!("expected data message");
        }
    }

    #[tokio::test]
    async fn test_transport_substream() {
        let client = MixnetClient::connect_new().await.unwrap();

        let (dialer_notify_inbound_tx, mut dialer_notify_inbound_rx) = unbounded_channel();
        let mut dialer_transport =
            NymTransport::new_with_notify_inbound(client, dialer_notify_inbound_tx)
                .await
                .unwrap();

        let client2 = MixnetClient::connect_new().await.unwrap();

        let (listener_notify_inbound_tx, mut listener_notify_inbound_rx) = unbounded_channel();
        let mut listener_transport =
            NymTransport::new_with_notify_inbound(client2, listener_notify_inbound_tx)
                .await
                .unwrap();
        let listener_multiaddr =
            nym_address_to_multiaddress(listener_transport.self_address).unwrap();
        assert_new_address_event(Pin::new(&mut dialer_transport)).await;
        assert_new_address_event(Pin::new(&mut listener_transport)).await;

        // dial the remote peer
        let mut dial = dialer_transport.dial(listener_multiaddr).unwrap();

        // poll the dial to send the connection request message
        assert!(poll_fn(|cx| Pin::new(&mut dial).as_mut().poll_unpin(cx))
            .now_or_never()
            .is_none());
        listener_notify_inbound_rx.recv().await.unwrap();

        // should receive the connection request from the mixnet and send the connection response
        let res = poll_fn(|cx| Pin::new(&mut listener_transport).as_mut().poll(cx)).await;
        let mut upgrade = match res {
            TransportEvent::Incoming {
                listener_id,
                upgrade,
                local_addr,
                send_back_addr,
            } => {
                assert_eq!(listener_id, listener_transport.listener_id);
                assert_eq!(local_addr, listener_transport.listen_addr);
                assert_eq!(send_back_addr, listener_transport.listen_addr);
                upgrade
            }
            _ => panic!("expected TransportEvent::Incoming, got {:?}", res),
        };
        dialer_notify_inbound_rx.recv().await.unwrap();

        // should receive the connection response from the mixnet
        assert!(
            poll_fn(|cx| Pin::new(&mut dialer_transport).as_mut().poll(cx))
                .now_or_never()
                .is_none()
        );
        info!("waiting for connections...");

        // should be able to resolve the connections now
        let (_, mut listener_conn) = poll_fn(|cx| Pin::new(&mut upgrade).as_mut().poll_unpin(cx))
            .now_or_never()
            .unwrap()
            .unwrap();
        let (_, mut dialer_conn) = poll_fn(|cx| Pin::new(&mut dial).as_mut().poll_unpin(cx))
            .now_or_never()
            .unwrap()
            .unwrap();
        info!("connections established");

        // initiate a new substream from the dialer
        let mut dialer_substream =
            poll_fn(|cx| Pin::new(&mut dialer_conn).as_mut().poll_outbound(cx))
                .await
                .unwrap();
        listener_notify_inbound_rx.recv().await.unwrap();

        // accept the substream on the listener
        assert!(
            poll_fn(|cx| Pin::new(&mut listener_transport).as_mut().poll(cx))
                .now_or_never()
                .is_none()
        );
        poll_fn(|cx| Pin::new(&mut listener_conn).as_mut().poll(cx)).now_or_never();

        // poll recipient's poll_inbound to receive the substream; sends a response to the sender
        let mut listener_substream =
            poll_fn(|cx| Pin::new(&mut listener_conn).as_mut().poll_inbound(cx))
                .now_or_never()
                .unwrap()
                .unwrap();
        info!("got listener substream");
        dialer_notify_inbound_rx.recv().await.unwrap();

        // poll sender to finalize the substream
        assert!(
            poll_fn(|cx| Pin::new(&mut dialer_transport).as_mut().poll(cx))
                .now_or_never()
                .is_none()
        );
        poll_fn(|cx| Pin::new(&mut dialer_conn).as_mut().poll(cx)).now_or_never();
        info!("got dialer substream");

        // write message from dialer to listener
        send_and_receive_substream_message(
            b"hello world".to_vec(),
            Pin::new(&mut dialer_substream),
            Pin::new(&mut listener_substream),
            Pin::new(&mut listener_transport),
            Pin::new(&mut listener_conn),
            &mut listener_notify_inbound_rx,
        )
        .await;

        // write message from listener to dialer
        send_and_receive_substream_message(
            b"hello back".to_vec(),
            Pin::new(&mut listener_substream),
            Pin::new(&mut dialer_substream),
            Pin::new(&mut dialer_transport),
            Pin::new(&mut dialer_conn),
            &mut dialer_notify_inbound_rx,
        )
        .await;

        // close the substream from the dialer side
        info!("closing dialer substream");
        dialer_substream.close().await.unwrap();
        listener_notify_inbound_rx.recv().await.unwrap();
        info!("dialer substream closed");

        // assert we can't read or write to either substream
        dialer_substream.write_all(b"hello").await.unwrap_err();
        // poll listener transport and conn to receive the substream close message
        poll_fn(|cx| Pin::new(&mut listener_transport).as_mut().poll(cx)).now_or_never();
        poll_fn(|cx| Pin::new(&mut listener_conn).as_mut().poll(cx)).now_or_never();
        listener_substream.write_all(b"hello").await.unwrap_err();
        let mut buf = vec![0u8; 5];
        dialer_substream.read(&mut buf).await.unwrap_err();
        listener_substream.read(&mut buf).await.unwrap_err();
        dialer_substream.close().await.unwrap_err();
        listener_substream.close().await.unwrap_err();
    }

    async fn send_and_receive_substream_message(
        data: Vec<u8>,
        mut sender_substream: Pin<&mut Substream>,
        mut recipient_substream: Pin<&mut Substream>,
        mut recipient_transport: Pin<&mut NymTransport>,
        mut recipient_conn: Pin<&mut Connection>,
        recipient_notify_inbound_rx: &mut UnboundedReceiver<()>,
    ) {
        // write message
        sender_substream.write_all(&data).await.unwrap();
        recipient_notify_inbound_rx.recv().await.unwrap();

        // poll recipient for message
        poll_fn(|cx| recipient_transport.as_mut().poll(cx)).now_or_never();
        poll_fn(|cx| recipient_conn.as_mut().poll(cx)).now_or_never();
        let mut buf = vec![0u8; data.len()];
        let n = recipient_substream.read(&mut buf).await.unwrap();
        assert_eq!(n, data.len());
        assert_eq!(buf, data[..]);
    }

    #[tokio::test]
    async fn test_transport_timeout() {
        let client = MixnetClient::connect_new().await.unwrap();

        let (dialer_notify_inbound_tx, _) = unbounded_channel();
        let mut dialer_transport =
            NymTransport::new_with_notify_inbound(client, dialer_notify_inbound_tx)
                .await
                .unwrap()
                .with_timeout(std::time::Duration::from_millis(100));

        // mock a transport that will never resolve the connection.
        let empty_addr = Multiaddr::from_str(
            "/nym/Hmer6Ndt3PV13YW53HM8ri4NvqqtfDQUQBhzvKqb1dag.2g478dyxtrQXGWc1Mk2VEqdPcWXpz7EhAcjhdAJtVZdA@AnnYnEtBjB2a5sHmeRCnBq43qxyHDf95Bqd7cwQyKNLR"
        )
        .expect("unable to parse multiaddress");

        let dial = dialer_transport.dial(empty_addr).unwrap();
        assert!(dial
            .await
            .expect_err("should have timed out")
            .to_string()
            .contains("dial timed out"));
    }
}
