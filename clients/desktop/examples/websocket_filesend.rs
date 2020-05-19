use futures::{SinkExt, StreamExt};
use nym_client::client::Recipient;
use nym_client::websocket::{BinaryClientRequest, ClientRequest, ServerResponse};
use std::convert::TryFrom;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() {
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

    let recipient = match response {
        ServerResponse::SelfAddress { address } => Recipient::try_from_string(address).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };
    println!("our full address is: {}", recipient.to_string());

    let read_data = std::fs::read("examples/dummy_file").unwrap();

    let send_request = BinaryClientRequest::Send {
        recipient,
        data: read_data,
    };

    println!("sending content of 'dummy_file' over the mix network...");
    ws_stream.send(send_request.into()).await.unwrap();

    let raw_send_confirmation = ws_stream.next().await.unwrap().unwrap();
    let _send_confirmation = match raw_send_confirmation {
        Message::Text(txt_msg) => match ServerResponse::try_from(txt_msg).unwrap() {
            ServerResponse::Send => (),
            _ => panic!("received an unexpected response type!"),
        },
        _ => panic!("received an unexpected response type!"),
    };

    println!("waiting to receive the 'dummy_file' from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Binary(bin_payload) => bin_payload,
        _ => panic!("received an unexpected response type!"),
    };

    println!("writing the file back to the disk!");
    std::fs::write("examples/received_file", message).unwrap();
}
