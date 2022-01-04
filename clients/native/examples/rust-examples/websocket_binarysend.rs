use futures::{SinkExt, StreamExt};
use nymsphinx::addressing::clients::Recipient;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

// just helpers functions that work in this very particular context because we are sending to ourselves
// and hence will always get a response back (i.e. the message we sent)
async fn send_message_and_get_response(
    ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    req: Vec<u8>,
) -> ServerResponse {
    ws_stream.send(Message::Binary(req)).await.unwrap();
    let raw_message = ws_stream.next().await.unwrap().unwrap();
    match raw_message {
        Message::Binary(bin_payload) => ServerResponse::deserialize(&bin_payload).unwrap(),
        _ => panic!("received an unexpected response type!"),
    }
}

async fn get_self_address(ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Recipient {
    let self_address_request = ClientRequest::SelfAddress.serialize();
    let response = send_message_and_get_response(ws_stream, self_address_request).await;

    match response {
        ServerResponse::SelfAddress(recipient) => recipient,
        _ => panic!("received an unexpected response!"),
    }
}

async fn send_file_with_reply() {
    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient.to_string());

    let read_data = std::fs::read("examples/dummy_file").unwrap();

    let send_request = ClientRequest::Send {
        recipient,
        message: read_data,
        with_reply_surb: true,
    };

    println!("sending content of 'dummy_file' over the mix network...");
    let response = send_message_and_get_response(&mut ws_stream, send_request.serialize()).await;

    let received = match response {
        ServerResponse::Received(received) => received,
        _ => panic!("received an unexpected response!"),
    };

    println!("writing the file back to the disk!");
    std::fs::write("examples/received_file_withreply", received.message).unwrap();

    let reply_message = b"hello from reply SURB! - thanks for sending me the file!".to_vec();
    let reply_request = ClientRequest::Reply {
        message: reply_message.clone(),
        reply_surb: received.reply_surb.unwrap(),
    };

    println!(
        "sending {:?} (using reply SURB!) over the mix network...",
        String::from_utf8(reply_message).unwrap()
    );
    let response = send_message_and_get_response(&mut ws_stream, reply_request.serialize()).await;
    let received = match response {
        ServerResponse::Received(received) => received,
        _ => panic!("received an unexpected response!"),
    };

    println!(
        "received {:#?} from the mix network!",
        String::from_utf8(received.message).unwrap()
    );
}

async fn send_file_without_reply() {
    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient.to_string());

    let read_data = std::fs::read("examples/dummy_file").unwrap();

    let send_request = ClientRequest::Send {
        recipient,
        message: read_data,
        with_reply_surb: false,
    };

    println!("sending content of 'dummy_file' over the mix network...");
    let response = send_message_and_get_response(&mut ws_stream, send_request.serialize()).await;

    let received = match response {
        ServerResponse::Received(received) => received,
        _ => panic!("received an unexpected response!"),
    };

    println!("writing the file back to the disk!");
    std::fs::write("examples/received_file_noreply", received.message).unwrap();
}

#[tokio::main]
async fn main() {
    println!("#############################");
    println!("Example without using replies");

    send_file_without_reply().await;

    println!("\n\n#############################");
    println!("Example using replies");

    send_file_with_reply().await;
}
