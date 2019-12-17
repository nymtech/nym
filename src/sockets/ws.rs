use crate::clients::InputMessage;
use futures::channel::mpsc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::stream::Stream;
use futures::Future;
use futures::{SinkExt, StreamExt};
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sphinx::route::Destination;
use std::net::SocketAddr;
use tokio::runtime::Runtime;
use tokio_tungstenite::accept_async;
use tungstenite::protocol::Message;
use std::time::Duration;

struct Connection {
    address: SocketAddr,
    rx: UnboundedReceiver<Message>,
    tx: UnboundedSender<Message>,
}

#[derive(Debug)]
enum WebSocketError {
    InvalidDestinationEncoding,
    InvalidDestinationLength,
}

impl From<hex::FromHexError> for WebSocketError {
    fn from(_: FromHexError) -> Self {
        use WebSocketError::*;

        InvalidDestinationEncoding
    }
}


#[derive(Serialize, Deserialize, Debug)]
struct ClientMessageJSON {
    message: String,
    recipient_address: String,
}

fn dummy_response() -> Message {
    Message::Text("foomp".to_string())
}

async fn handle_connection(conn: Connection) {
    let mut conn = conn;
    while let Some(msg) = conn.rx.next().await {
        println!("Received a message from {}: {}", conn.address, msg);
        // TODO: currently only hardcoded sends are supported
        let parsed_message = parse_message(msg);
        println!("parsed: {:?}", parsed_message);
        println!("test async pre wait");

        tokio::time::delay_for(Duration::from_secs(2)).await;

        println!("test async post wait");
        conn
            .tx
            .unbounded_send(dummy_response())
            .expect("Failed to forward message");
    }
}

// Proves we can call Rust methods from the websocket listener. Re-route it to wherever JS puts
// the `send_message` functionality.
fn parse_message(msg: Message) -> Result<InputMessage, WebSocketError> {
    let text_msg = match msg {
        Message::Text(msg) => msg,
        Message::Binary(_) => panic!("binary messages are not supported!"),
        Message::Close(_) => panic!("todo: handle close!"),
        _ => panic!("Other types of messages are also unsupported!"),
    };

    let raw_msg: ClientMessageJSON = serde_json::from_str(&text_msg).unwrap();
    let address_vec = hex::decode(raw_msg.recipient_address)?;

    if address_vec.len() != 32 {
        return Err(WebSocketError::InvalidDestinationLength);
    }

    let mut address = [0; 32];
    address.copy_from_slice(&address_vec);

    let dummy_surb = [0; 16];

    Ok(InputMessage(
        Destination::new(address, dummy_surb),
        raw_msg.message.into_bytes(),
    ))
}

async fn accept_connection(stream: tokio::net::TcpStream) {
    let address = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", address);

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", address);

    // Create a channel for our stream, which other sockets will use to
    // send us messages. Then register our address with the stream to send
    // data to us.
    let (msg_tx, msg_rx) = futures::channel::mpsc::unbounded();
    let (response_tx, mut response_rx) = futures::channel::mpsc::unbounded();
    let conn = Connection {
        address,
        rx: msg_rx,
        tx: response_tx,
    };
    tokio::spawn(handle_connection(conn));

    while let Some(message) = ws_stream.next().await {
        let message = message.expect("Failed to get request");
        msg_tx
            .unbounded_send(message)
            .expect("Failed to forward request");
        if let Some(resp) = response_rx.next().await {
            ws_stream.send(resp).await.expect("Failed to send response");
        }
    }
}



pub fn start(address: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let mut rt = Runtime::new()?;

    rt.block_on(async {
        let mut listener = tokio::net::TcpListener::bind(address).await?;

        while let Ok((stream, _)) = listener.accept().await {
            // TODO: should it rather be rt.spawn?
            tokio::spawn(accept_connection(stream));
        }

        eprintln!("The websocket went kaput...");
        Ok(())
    })
}

