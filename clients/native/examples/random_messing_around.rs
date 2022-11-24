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
        ServerResponse::SelfAddress(recipient) => *recipient,
        res => panic!("received an unexpected response! - {:?}", res),
    }
}

async fn send_file_without_reply() {
    let uri = "ws://localhost:1977";
    let (mut ws_stream, _) = connect_async(uri).await.unwrap();

    let recipient = get_self_address(&mut ws_stream).await;
    println!("our full address is: {}", recipient);

    let read_data = vec![42u8; 100_000];

    let send_request = ClientRequest::SendAnonymous {
        recipient,
        message: read_data,
        reply_surbs: 3,
        connection_id: 0,
    };

    println!("sending content of 'dummy_file' over the mix network...");
    let response = send_message_and_get_response(&mut ws_stream, send_request.serialize()).await;

    let received = match response {
        ServerResponse::Received(received) => received,
        res => panic!("received an unexpected response! - {:?}", res),
    };

    let sender_tag = received.sender_tag.unwrap();
    println!(
        "received response was {} in length and contained the following sender_tag: {:?}",
        received.message.len(),
        sender_tag
    );

    let send_reply_req = ClientRequest::Reply {
        // message: vec![5, 6, 7, 8],
        message: vec![42u8; 100000],
        sender_tag,
        connection_id: 0,
    };
    let response = send_message_and_get_response(&mut ws_stream, send_reply_req.serialize()).await;

    let received2 = match response {
        ServerResponse::Received(received) => received,
        res => panic!("received an unexpected response! - {:?}", res),
    };

    println!(
        "received reply-backed response was: {} in length",
        received2.message.len()
    );

    // println!("writing the file back to the disk!");
    // std::fs::write("examples/received_file_noreply", received.message).unwrap();
}

#[tokio::main]
async fn main() {
    println!("#############################");
    println!("Example without using replies");

    send_file_without_reply().await;

    // println!("\n\n#############################");
    // println!("Example using replies");
    //
    // send_file_with_reply().await;
}
