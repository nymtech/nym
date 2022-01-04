use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

// PREFACE: in practice I don't see why you would ever want to use text api while in Rust, but example
// is here for the completion sake

// just helpers functions that work in this very particular context because we are sending to ourselves
// and hence will always get a response back (i.e. the message we sent)
async fn send_message_and_get_json_response(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    text_req: String,
) -> serde_json::Value {
    ws_stream.send(Message::Text(text_req)).await.unwrap();
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    match raw_message {
        Message::Text(txt_msg) => serde_json::from_str(&txt_msg).unwrap(),
        _ => panic!("received an unexpected response type!"),
    }
}

async fn get_self_address(ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> String {
    let self_address_request = json!({ "type": "selfAddress" }).to_string();
    let response = send_message_and_get_json_response(ws_stream, self_address_request).await;

    response["address"].as_str().unwrap().to_string()
}

async fn send_text_with_reply() {
    let message = "Hello Nym!".to_string();

    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient.to_string());

    let send_request = json!({
        "type" : "send",
        "recipient": recipient,
        "message": message,
        "withReplySurb": true,
    });

    println!(
        "sending {:?} (*with* reply SURB) over the mix network...",
        message
    );
    let response =
        send_message_and_get_json_response(&mut ws_stream, send_request.to_string()).await;

    let reply_message = "hello from reply SURB!";
    let reply_request = json!({
        "type": "reply",
        "message": reply_message,
        "replySurb": response["replySurb"]
    });

    println!(
        "sending {:?} (using reply SURB!) over the mix network...",
        reply_message
    );

    let response =
        send_message_and_get_json_response(&mut ws_stream, reply_request.to_string()).await;
    println!("received {:#?} from the mix network!", response.to_string());
}

async fn send_text_without_reply() {
    let message = "Hello Nym!".to_string();

    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient.to_string());

    let send_request = json!({
        "type" : "send",
        "recipient": recipient,
        "message": message,
        "withReplySurb": false,
    });

    println!(
        "sending {:?} (*without* reply SURB) over the mix network...",
        message
    );
    let response =
        send_message_and_get_json_response(&mut ws_stream, send_request.to_string()).await;

    println!("received {:#?} from the mix network!", response.to_string());
}

#[tokio::main]
async fn main() {
    println!("#############################");
    println!("Example without using replies");

    send_text_without_reply().await;

    println!("\n\n#############################");
    println!("Example using replies");

    send_text_with_reply().await;
}
