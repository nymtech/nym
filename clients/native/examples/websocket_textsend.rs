use futures::{SinkExt, StreamExt};
use nym_client::websocket::{ClientTextRequest, ServerTextResponse};
use std::convert::TryFrom;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

async fn send_text_without_reply() {
    let message = "Hello Nym!".to_string();

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

    let self_address = match response {
        ServerTextResponse::SelfAddress { address } => address,
        _ => panic!("received an unexpected response type!"),
    };
    println!("our address is: {}", self_address.clone());

    let send_request = ClientTextRequest::Send {
        message: message.clone(),
        recipient: self_address,
        with_reply_surb: false,
    };
    println!(
        "sending {:?} (*without reply SURB) over the mix network...",
        message
    );
    ws_stream.send(send_request.into()).await.unwrap();

    println!("waiting to receive a message from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let response = match message {
        ServerTextResponse::Received(received_message) => received_message,
        _ => panic!("received an unexpected response type!"),
    };

    println!("received {:#?} from the mix network!", response);
}

async fn send_text_with_reply() {
    let message = "Hello Nym!".to_string();

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

    let self_address = match response {
        ServerTextResponse::SelfAddress { address } => address,
        _ => panic!("received an unexpected response type!"),
    };
    println!("our address is: {}", self_address.clone());

    let send_request = ClientTextRequest::Send {
        message: message.clone(),
        recipient: self_address,
        with_reply_surb: true,
    };
    println!(
        "sending {:?} (*wiht* reply SURB) over the mix network...",
        message
    );
    ws_stream.send(send_request.into()).await.unwrap();

    println!("waiting to receive a message from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let response = match message {
        ServerTextResponse::Received(received_message) => received_message,
        _ => panic!("received an unexpected response type!"),
    };

    println!("received {:#?} from the mix network!", response);

    let reply_message = "hello from reply SURB!";
    let reply_request = ClientTextRequest::Reply {
        message: reply_message.to_string(),
        reply_surb: response.reply_surb.unwrap(),
    };

    println!(
        "sending {:?} (using reply SURB!) over the mix network...",
        reply_message
    );
    ws_stream.send(reply_request.into()).await.unwrap();

    println!("waiting to receive a message from the mix network...");
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    let response = match message {
        ServerTextResponse::Received(received_message) => received_message,
        _ => panic!("received an unexpected response type!"),
    };

    println!("received {:#?} from the mix network!", response);
}

#[tokio::main]
async fn main() {
    // send_text_without_reply().await;
    send_text_with_reply().await;
}
