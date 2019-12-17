use crate::clients::InputMessage;
use futures::channel::mpsc;
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sphinx::route::Destination;
use std::net::SocketAddr;
use ws::{listen, CloseCode, Handler, Message, Sender};

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

struct Server {
    out: Sender,
    input_tx: mpsc::UnboundedSender<InputMessage>,
}

impl Handler for Server {
    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        let parsed_msg = Server::parse_message(msg.clone());

        println!("msg: {:?}", parsed_msg);
        // Echo the message back
        //        self.out.send(msg)
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => println!("The client is done with the connection."),
            CloseCode::Away => println!("The client is leaving the site."),
            _ => {
                println!("The client encountered an error: {}", reason);
            }
        }
    }
}

impl Server {
    // Proves we can call Rust methods from the websocket listener. Re-route it to wherever JS puts
    // the `send_message` functionality.
    fn parse_message(msg: Message) -> Result<InputMessage, WebSocketError> {
        let text_msg = match msg {
            Message::Text(msg) => msg,
            Message::Binary(_) => panic!("binary messages are not supported!"),
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
}

pub fn start(socket_address: SocketAddr, input_tx: mpsc::UnboundedSender<InputMessage>) {
    //    listen("127.0.0.1:3012", |out| {
    //        async move |msg| {
    //            out.send(msg)
    //        }
    //    });

    //    listen(socket_address, |out| Server { out, input_tx }).unwrap()
}

#[derive(Serialize, Deserialize, Debug)]
struct ClientMessageJSON {
    message: String,
    recipient_address: String,
}
