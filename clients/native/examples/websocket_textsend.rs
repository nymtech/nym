use futures::{SinkExt, StreamExt};
use nym_client::websocket::{ClientTextRequest, ReceivedTextMessage, ServerTextResponse};
use std::convert::TryFrom;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};

// just helpers functions that work in this very particular context because we are sending to ourselves
// and hence will always get a response back (i.e. the message we sent)
async fn send_message_and_get_response<M: Into<Message>>(
    ws_stream: &mut WebSocketStream<TcpStream>,
    req: M,
) -> ReceivedTextMessage {
    ws_stream.send(req.into()).await.unwrap();
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    let message = match raw_message {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    match message {
        ServerTextResponse::Received(received_message) => received_message,
        _ => panic!("received an unexpected response type!"),
    }
}

async fn get_self_address(ws_stream: &mut WebSocketStream<TcpStream>) -> String {
    let self_address_request = ClientTextRequest::SelfAddress;
    ws_stream.send(self_address_request.into()).await.unwrap();

    let raw_response = ws_stream.next().await.unwrap().unwrap();
    // what we received now is just a json, but we know it's exact format
    // so might as well use that
    let response = match raw_response {
        Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    };

    match response {
        ServerTextResponse::SelfAddress { address } => address,
        _ => panic!("received an unexpected response type!"),
    }
}

async fn send_text_with_reply() {
    let message = "Hello Nym!".to_string();

    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient.to_string());

    let send_request = ClientTextRequest::Send {
        message: message.clone(),
        recipient,
        with_reply_surb: true,
    };
    println!(
        "sending {:?} (*with* reply SURB) over the mix network...",
        message
    );

    let response = send_message_and_get_response(&mut ws_stream, send_request).await;
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

    let response = send_message_and_get_response(&mut ws_stream, reply_request).await;
    println!("received {:#?} from the mix network!", response);
}

async fn send_text_without_reply() {
    let message = "Hello Nym!".to_string();

    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient.to_string());

    let send_request = ClientTextRequest::Send {
        message: message.clone(),
        recipient,
        with_reply_surb: false,
    };
    println!(
        "sending {:?} (*without reply SURB) over the mix network...",
        message
    );
    let response = send_message_and_get_response(&mut ws_stream, send_request).await;

    println!("received {:#?} from the mix network!", response);
}

#[tokio::main]
async fn main() {
    // send_text_without_reply().await;
    send_text_with_reply().await;
}
