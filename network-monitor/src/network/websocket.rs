use futures::{SinkExt, StreamExt};
use log::info;
use nymsphinx::addressing::clients::Recipient;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

pub struct Connection {
    uri: String,
    websocket_stream: WebSocketStream<TcpStream>,
}

impl Connection {
    pub async fn new(uri: &str) -> Connection {
        let ws_stream = match connect_async(uri).await {
            Ok((ws_stream, _)) => {
                info!("* connected to local websocket server at {}", uri);
                ws_stream
            }
            Err(_e) => {
                panic!("Error: websocket connection attempt failed, is the Nym client running?");
            }
        };
        Connection {
            uri: String::from(uri),
            websocket_stream: ws_stream,
        }
    }

    pub async fn get_self_address(self) -> Recipient {
        let self_address_request = ClientRequest::SelfAddress.serialize();
        let response = self
            .send_message_and_get_response(self_address_request)
            .await;

        match response {
            ServerResponse::SelfAddress(recipient) => recipient,
            _ => panic!("received an unexpected response!"),
        }
    }

    // just helpers functions that work in this very particular context because we are sending to ourselves
    // and hence will always get a response back (i.e. the message we sent)
    async fn send_message_and_get_response(self, req: Vec<u8>) -> ServerResponse {
        let mut stream = self.websocket_stream;
        stream.send(Message::Binary(req)).await.unwrap();
        let raw_message = stream.next().await.unwrap().unwrap();
        match raw_message {
            Message::Binary(bin_payload) => ServerResponse::deserialize(&bin_payload).unwrap(),
            _ => panic!("received an unexpected response type!"),
        }
    }
}
