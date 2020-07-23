use futures::{SinkExt, StreamExt};
use nym_client::websocket::{
    BinaryClientRequest, BinaryServerResponse, ClientTextRequest, ServerTextResponse,
};
use nymsphinx::addressing::clients::Recipient;
use std::convert::TryFrom;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

async fn send_file_with_reply() {
    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let self_address_request = ClientTextRequest::SelfAddress;
    ws_stream.send(self_address_request.into()).await.unwrap();

    let raw_response = ws_stream.next().await.unwrap().unwrap();
    // what we received now is just a json, but we know it's exact format
    // so might as well use that
    let response = match raw_response {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let recipient = match response {
        ServerTextResponse::SelfAddress { address } => Recipient::try_from_string(address).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };
    println!("our full address is: {}", recipient.to_string());

    let read_data = std::fs::read("examples/dummy_file").unwrap();

    let send_request = BinaryClientRequest::Send {
        recipient,
        data: read_data,
        with_reply_surb: true,
    };

    println!("sending content of 'dummy_file' over the mix network...");
    ws_stream.send(send_request.into()).await.unwrap();

    println!("waiting to receive the 'dummy_file' from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Binary(bin_payload) => BinaryServerResponse::try_from_bytes(&bin_payload).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let response = match message {
        BinaryServerResponse::Received(received_message) => received_message,
    };

    println!("writing the file back to the disk!");
    std::fs::write("examples/received_file_withreply", response.message).unwrap();

    let reply_message = b"hello from reply SURB! - thanks for sending me the file!".to_vec();
    let reply_request = BinaryClientRequest::Reply {
        message: reply_message.clone(),
        reply_surb: response.reply_SURB.unwrap(),
    };

    println!(
        "sending {:?} (using reply SURB!) over the mix network...",
        String::from_utf8(reply_message).unwrap()
    );
    ws_stream.send(reply_request.into()).await.unwrap();

    println!("waiting to receive a message from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Binary(bin_payload) => BinaryServerResponse::try_from_bytes(&bin_payload).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let response = match message {
        BinaryServerResponse::Received(received_message) => received_message,
    };

    println!(
        "received {:#?} from the mix network!",
        String::from_utf8(response.message).unwrap()
    );
}

async fn send_file_without_reply() {
    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let self_address_request = ClientTextRequest::SelfAddress;
    ws_stream.send(self_address_request.into()).await.unwrap();

    let raw_response = ws_stream.next().await.unwrap().unwrap();
    // what we received now is just a json, but we know it's exact format
    // so might as well use that
    let response = match raw_response {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let recipient = match response {
        ServerTextResponse::SelfAddress { address } => Recipient::try_from_string(address).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };
    println!("our full address is: {}", recipient.to_string());

    let read_data = std::fs::read("examples/dummy_file").unwrap();

    let send_request = BinaryClientRequest::Send {
        recipient,
        data: read_data,
        with_reply_surb: false,
    };

    println!("sending content of 'dummy_file' over the mix network...");
    ws_stream.send(send_request.into()).await.unwrap();

    println!("waiting to receive the 'dummy_file' from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Binary(bin_payload) => BinaryServerResponse::try_from_bytes(&bin_payload).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let response = match message {
        BinaryServerResponse::Received(received_message) => received_message,
    };

    println!("writing the file back to the disk!");
    std::fs::write("examples/received_file_noreply", response.message).unwrap();
}

#[tokio::main]
async fn main() {
    // send_file_without_reply().await;
    send_file_with_reply().await
}
