// use futures::{SinkExt, StreamExt};
// use nym_client::websocket::{
//     BinaryClientRequest, BinaryServerResponse, ClientTextRequest, ServerTextResponse,
// };
// use nymsphinx::addressing::clients::Recipient;
// use nymsphinx::receiver::ReconstructedMessage;
// use std::convert::TryFrom;
// use tokio::net::TcpStream;
// use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
//
// // just helpers functions that work in this very particular context because we are sending to ourselves
// // and hence will always get a response back (i.e. the message we sent)
// async fn send_message_and_get_response<M: Into<Message>>(
//     ws_stream: &mut WebSocketStream<TcpStream>,
//     req: M,
// ) -> ReconstructedMessage {
//     ws_stream.send(req.into()).await.unwrap();
//     let raw_message = ws_stream.next().await.unwrap().unwrap();
//     let message = match raw_message {
//         Message::Binary(bin_payload) => BinaryServerResponse::try_from_bytes(&bin_payload).unwrap(),
//         _ => panic!("received an unexpected response type!"),
//     };
//
//     match message {
//         BinaryServerResponse::Received(received_message) => received_message,
//     }
// }
//
// async fn get_self_address(ws_stream: &mut WebSocketStream<TcpStream>) -> Recipient {
//     let self_address_request = ClientTextRequest::SelfAddress;
//     ws_stream.send(self_address_request.into()).await.unwrap();
//
//     let raw_response = ws_stream.next().await.unwrap().unwrap();
//     // what we received now is just a json, but we know it's exact format
//     // so might as well use that
//     let response = match raw_response {
//         Message::Text(txt_msg) => ServerTextResponse::try_from(txt_msg).unwrap(),
//         _ => panic!("received an unexpected response type!"),
//     };
//
//     match response {
//         ServerTextResponse::SelfAddress { address } => Recipient::try_from_string(address).unwrap(),
//         _ => panic!("received an unexpected response type!"),
//     }
// }
//
// async fn send_file_with_reply() {
//     let uri = "ws://localhost:1977";
//     let (mut ws_stream, _) = connect_async(uri).await.unwrap();
//
//     let recipient = get_self_address(&mut ws_stream).await;
//     println!("our full address is: {}", recipient.to_string());
//
//     let read_data = std::fs::read("examples/dummy_file").unwrap();
//
//     let send_request = BinaryClientRequest::Send {
//         recipient,
//         data: read_data,
//         with_reply_surb: true,
//     };
//
//     println!("sending content of 'dummy_file' over the mix network...");
//     let response = send_message_and_get_response(&mut ws_stream, send_request).await;
//
//     println!("writing the file back to the disk!");
//     std::fs::write("examples/received_file_withreply", response.message).unwrap();
//
//     let reply_message = b"hello from reply SURB! - thanks for sending me the file!".to_vec();
//     let reply_request = BinaryClientRequest::Reply {
//         message: reply_message.clone(),
//         reply_surb: response.reply_SURB.unwrap(),
//     };
//
//     println!(
//         "sending {:?} (using reply SURB!) over the mix network...",
//         String::from_utf8(reply_message).unwrap()
//     );
//     let response = send_message_and_get_response(&mut ws_stream, reply_request).await;
//
//     println!(
//         "received {:#?} from the mix network!",
//         String::from_utf8(response.message).unwrap()
//     );
// }
//
// async fn send_file_without_reply() {
//     let uri = "ws://localhost:1977";
//     let (mut ws_stream, _) = connect_async(uri).await.unwrap();
//
//     let recipient = get_self_address(&mut ws_stream).await;
//     println!("our full address is: {}", recipient.to_string());
//
//     let read_data = std::fs::read("examples/dummy_file").unwrap();
//
//     let send_request = BinaryClientRequest::Send {
//         recipient,
//         data: read_data,
//         with_reply_surb: false,
//     };
//
//     println!("sending content of 'dummy_file' over the mix network...");
//     let response = send_message_and_get_response(&mut ws_stream, send_request).await;
//
//     println!("writing the file back to the disk!");
//     std::fs::write("examples/received_file_noreply", response.message).unwrap();
// }
//
#[tokio::main]
async fn main() {
    // send_file_without_reply().await;
    // send_file_with_reply().await
}
