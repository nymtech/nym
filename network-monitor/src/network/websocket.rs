use futures::{SinkExt, StreamExt};
use nymsphinx::addressing::clients::Recipient;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

pub struct Connection {
    uri: String,
}

impl Connection {
    pub fn new(uri: &str) -> Connection {
        Connection {
            uri: String::from(uri),
        }
    }

    pub async fn connect(&self) -> Result<WebSocketStream<TcpStream>, WebsocketConnectionError> {
        match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => Ok(ws_stream),
            Err(_e) => Err(WebsocketConnectionError::ConnectionNotEstablished),
        }
    }
}

pub async fn get_self_address(ws_stream: &mut WebSocketStream<TcpStream>) -> Recipient {
    let self_address_request = ClientRequest::SelfAddress.serialize();
    let response = send_message_and_get_response(ws_stream, self_address_request).await;

    match response {
        ServerResponse::SelfAddress(recipient) => recipient,
        _ => panic!("received an unexpected response!"),
    }
}

// just helpers functions that work in this very particular context because we are sending to ourselves
// and hence will always get a response back (i.e. the message we sent)
async fn send_message_and_get_response(
    ws_stream: &mut WebSocketStream<TcpStream>,
    req: Vec<u8>,
) -> ServerResponse {
    ws_stream.send(Message::Binary(req)).await.unwrap();
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    match raw_message {
        Message::Binary(bin_payload) => ServerResponse::deserialize(&bin_payload).unwrap(),
        _ => panic!("received an unexpected response type!"),
    }
}
#[derive(Debug)]
pub enum WebsocketConnectionError {
    ConnectionNotEstablished,
}
