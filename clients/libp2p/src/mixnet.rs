use futures::{
    pin_mut, select,
    stream::{SplitSink, SplitStream},
};
use futures::{FutureExt, SinkExt, StreamExt};
use nym_client_websocket_requests::{requests::ClientRequest, responses::ServerResponse};
use nym_sphinx::addressing::clients::Recipient;
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::debug;

use crate::error::Error;
use crate::message::*;

/// initialize_mixnet initializes a read/write connection to a Nym websockets endpoint.
/// It starts a task that listens for inbound messages from the endpoint and writes outbound messages to the endpoint.
pub(crate) async fn initialize_mixnet(
    uri: &String,
    notify_inbound_tx: Option<UnboundedSender<()>>,
) -> Result<
    (
        Recipient,
        UnboundedReceiver<InboundMessage>,
        UnboundedSender<OutboundMessage>,
    ),
    Error,
> {
    let (mut ws_stream, _) = connect_async(uri)
        .await
        .map_err(Error::WebsocketStreamError)?;

    let recipient = get_self_address(&mut ws_stream).await?;

    // a channel of inbound messages from the mixnet..
    // the transport reads from (listens) to the inbound_rx.
    // TODO: this is probably a DOS vector; we should limit the size of the channel.
    let (inbound_tx, inbound_rx) = unbounded_channel::<InboundMessage>();

    // a channel of outbound messages to be written to the mixnet.
    // the transport writes to outbound_tx.
    let (outbound_tx, mut outbound_rx) = unbounded_channel::<OutboundMessage>();

    let (mut sink, mut stream) = ws_stream.split();

    tokio::task::spawn(async move {
        loop {
            let t1 = check_inbound(&mut stream, &inbound_tx, &notify_inbound_tx).fuse();
            let t2 = check_outbound(&mut sink, &mut outbound_rx).fuse();

            pin_mut!(t1, t2);

            select! {
                _ = t1 => {},
                _ = t2 => {},
            };
        }
    });

    Ok((recipient, inbound_rx, outbound_tx))
}

async fn check_inbound(
    ws_stream: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    inbound_tx: &UnboundedSender<InboundMessage>,
    notify_inbound_tx: &Option<UnboundedSender<()>>,
) -> Result<(), Error> {
    if let Some(res) = ws_stream.next().await {
        if let Some(notify_tx) = notify_inbound_tx {
            notify_tx
                .send(())
                .map_err(|e| Error::InboundSendError(e.to_string()))?;
        }

        match res {
            Ok(msg) => return handle_inbound(msg, inbound_tx).await,
            Err(e) => return Err(Error::WebsocketStreamError(e)),
        }
    }

    Err(Error::WebsocketStreamReadNone)
}

async fn handle_inbound(
    msg: Message,
    inbound_tx: &UnboundedSender<InboundMessage>,
) -> Result<(), Error> {
    let res = parse_nym_message(msg)?;
    let msg_bytes = match res {
        ServerResponse::Received(msg_bytes) => msg_bytes,
        ServerResponse::Error(e) => return Err(Error::NymMessageError(e.to_string())),
        _ => return Err(Error::UnexpectedNymMessage),
    };
    let data = parse_message_data(&msg_bytes.message)?;
    inbound_tx
        .send(data)
        .map_err(|e| Error::InboundSendError(e.to_string()))?;
    Ok(())
}

async fn check_outbound(
    ws_sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    outbound_rx: &mut UnboundedReceiver<OutboundMessage>,
) -> Result<(), Error> {
    match outbound_rx.recv().await {
        Some(message) => write_bytes(ws_sink, message.recipient, &message.message.to_bytes()).await,
        None => Err(Error::RecvError),
    }
}

async fn write_bytes(
    ws_sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    recipient: Recipient,
    message: &[u8],
) -> Result<(), Error> {
    let nym_packet = ClientRequest::Send {
        recipient,
        message: message.to_vec(),
        connection_id: None,
    };

    ws_sink
        .send(Message::Binary(nym_packet.serialize()))
        .await
        .map_err(Error::WebsocketStreamError)?;

    debug!(
        "wrote message to mixnet: recipient: {:?}",
        recipient.to_string()
    );
    Ok(())
}

async fn get_self_address(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<Recipient, Error> {
    let self_address_request = ClientRequest::SelfAddress.serialize();
    ws_stream
        .send(Message::Binary(self_address_request))
        .await
        .map_err(Error::WebsocketStreamError)?;

    // loop until we receive the SelfAddress respone, since the next message might not
    // necessarily be the SelfAddress response.
    while let Some(raw_message) = ws_stream.next().await {
        let raw_message = raw_message.map_err(Error::WebsocketStreamError)?;
        let response = parse_nym_message(raw_message)?;
        return match response {
            ServerResponse::SelfAddress(recipient) => Ok(*recipient),
            ServerResponse::Error(e) => Err(Error::NymMessageError(e.to_string())),
            _ => continue,
        };
    }
    Err(Error::RecvError)
}

fn parse_nym_message(msg: Message) -> Result<ServerResponse, Error> {
    match msg {
        Message::Text(str) => ServerResponse::deserialize(&str.into_bytes())
            .map_err(|e| Error::NymMessageError(e.to_string())),
        Message::Binary(bytes) => {
            ServerResponse::deserialize(&bytes).map_err(|e| Error::NymMessageError(e.to_string()))
        }
        _ => Err(Error::UnknownNymMessage),
    }
}

#[cfg(test)]
mod test {
    use testcontainers::clients;

    use crate::message::{
        self, ConnectionId, Message, SubstreamId, SubstreamMessage, SubstreamMessageType,
        TransportMessage,
    };
    use crate::mixnet::initialize_mixnet;
    use crate::test_utils::create_nym_client;

    #[tokio::test]
    async fn test_mixnet_poll_inbound_and_outbound() {
        let docker_client = clients::Cli::default();
        let nym_id = "test_mixnet_poll_inbound_and_outbound";
        let (_container1, uri) = create_nym_client(&docker_client, nym_id);
        let (self_address, mut inbound_rx, outbound_tx) =
            initialize_mixnet(&uri, None).await.unwrap();
        let msg_inner = "hello".as_bytes();
        let substream_id = SubstreamId::generate();
        let msg = Message::TransportMessage(TransportMessage {
            nonce: 1, // arbitrary
            id: ConnectionId::generate(),
            message: SubstreamMessage::new_with_data(substream_id.clone(), msg_inner.to_vec()),
        });

        // send a message to ourselves through the mixnet
        let out_msg = message::OutboundMessage {
            message: msg,
            recipient: self_address,
        };

        outbound_tx.send(out_msg).unwrap();

        // receive the message from ourselves over the mixnet
        let received_msg = inbound_rx.recv().await.unwrap();
        if let Message::TransportMessage(recv_msg) = received_msg.0 {
            assert_eq!(substream_id, recv_msg.message.substream_id);
            if let SubstreamMessageType::Data(data) = recv_msg.message.message_type {
                assert_eq!(msg_inner, data.as_slice());
            } else {
                panic!("expected SubstreamMessage::Data")
            }
        } else {
            panic!("expected Message::TransportMessage")
        }
    }
}
