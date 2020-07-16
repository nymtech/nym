use futures::{SinkExt, StreamExt};
use nym_client::websocket::{ClientRequest, ServerResponse};
use std::convert::TryFrom;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() {
    let message = "Hello Nym!".to_string();

    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let self_address_request = ClientRequest::SelfAddress;
    ws_stream.send(self_address_request.into()).await.unwrap();

    let raw_response = ws_stream.next().await.unwrap().unwrap();
    // what we received now is just a json, but we know it's exact format
    // so might as well use that
    let response = match raw_response {
        Message::Text(txt_msg) => ServerResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let self_address = match response {
        ServerResponse::SelfAddress { address } => address,
        _ => panic!("received an unexpected response type!"),
    };
    println!("our address is: {}", self_address.clone());

    let send_request = ClientRequest::Send {
        message: message.clone(),
        recipient: self_address,
        with_reply_surb: false,
    };
    println!("sending {:?} over the mix network...", message);
    ws_stream.send(send_request.into()).await.unwrap();

    let raw_send_confirmation = ws_stream.next().await.unwrap().unwrap();
    let _send_confirmation = match raw_send_confirmation {
        Message::Text(txt_msg) => match ServerResponse::try_from(txt_msg).unwrap() {
            ServerResponse::Send => (),
            _ => panic!("received an unexpected response type!"),
        },
        _ => panic!("received an unexpected response type!"),
    };

    println!("waiting to receive a message from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Text(txt_msg) => txt_msg,
        _ => panic!("received an unexpected response type!"),
    };

    println!("received {:?} from the mix network!", message);
}
