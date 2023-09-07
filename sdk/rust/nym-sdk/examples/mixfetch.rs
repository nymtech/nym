extern crate core;

// use bytecodec::bytes::BytesEncoder;
// use bytecodec::bytes::RemainingBytesDecoder;
// use bytecodec::io::IoEncodeExt;
// use bytecodec::{DecodeExt, Encode};
// use httpcodec::{BodyDecoder, ResponseDecoder};
// use httpcodec::{
//     BodyEncoder, HeaderField, HttpVersion, Method, Request, RequestEncoder, RequestTarget,
// };
// use nym_sdk::mixnet;
// use nym_sdk::mixnet::{IncludedSurbs, Recipient};
// use nym_service_providers_common::interface::Serializable;
// use nym_socks5_requests::{
//     Socks5ProtocolVersion, Socks5ProviderRequest, Socks5Response, Socks5ResponseContent,
// };
// use std::time::Duration;

#[tokio::main]
async fn main() {
    unimplemented!()
    // nym_bin_common::logging::setup_logging();
    //
    // // Passing no config makes the client fire up an ephemeral session and figure shit out on its own
    // let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    //
    // // Be able to get our client address
    // let our_address = client.nym_address();
    // println!("Our client nym address is: {our_address}");
    //
    // // Send a message through the mixnet to ourselves
    // // client.send_str(*our_address, "hello there").await;
    //
    // // Build an HTTP GET request
    // let mut request = Request::new(
    //     Method::new("GET").unwrap(),
    //     RequestTarget::new("/.wellknown/wallet/validators.json").unwrap(),
    //     HttpVersion::V1_1,
    //     b"",
    // );
    // let mut headers = request.header_mut();
    // headers.add_field(HeaderField::new("Host", "nymtech.net").unwrap());
    //
    // // Set up an HTTP GET request, with headers and no payload
    // // let mut request = Request::new(
    // //     Method::new("GET").unwrap(),
    // //     RequestTarget::new("/package.json").unwrap(),
    // //     HttpVersion::V1_1,
    // //     b"",
    // // );
    // // let mut headers = request.header_mut();
    // // headers.add_field(HeaderField::new("Host", "localhost:3000").unwrap());
    //
    // // Encode as bytes
    // let mut encoder = RequestEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    // encoder.start_encoding(request).unwrap();
    // let mut buf = Vec::new();
    // encoder.encode_all(&mut buf).unwrap();
    //
    // println!("{:?}", buf);
    // println!("{}", String::from_utf8_lossy(&buf));
    //
    // let client_address = Recipient::try_from_base58_string("8YF6f8x17j3fviBdU87EGD9g9MAgn9DARxunwLEVM7Bm.4ydfpjbTjCmzj58hWdQjxU2gT6CRVnTbnKajr2hAGBBM@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW")
    //     .expect("address is valid");
    //
    // // Any old connection id will do
    // let conn_id = 13488440783042593894u64;
    //
    // // Wrap is SOCKS5 connect request
    // let socks5_connect = nym_socks5_requests::request::Socks5Request::new_connect(
    //     Socks5ProtocolVersion::Legacy,
    //     conn_id,
    //     "nymtech.net:443".to_string(),
    //     // "localhost:3000".to_string(),
    //     Some(*our_address),
    // );
    // nym_socks5_requests::request::Socks5Request::try_from_bytes(
    //     &socks5_connect.clone().into_bytes(),
    // )
    // .unwrap();
    //
    // let msg_connect = Socks5ProviderRequest::new_provider_data(
    //     nym_socks5_client_core::config::Config::default()
    //         .get_socks5()
    //         .get_provider_interface_version(),
    //     socks5_connect,
    // );
    //
    // client
    //     .send_bytes(
    //         client_address,
    //         msg_connect.into_bytes(),
    //         IncludedSurbs::ExposeSelfAddress,
    //     )
    //     .await;
    //
    // // Sleep to avoid weird packet ordering
    // tokio::time::sleep(Duration::from_millis(1000u64)).await;
    //
    // // Create an ordered message
    // let ordered_msg = OrderedMessage {
    //     data: buf,
    //     index: 0,
    // };
    //
    // let buf2 = ordered_msg.into_bytes();
    // println!("socks5 with payload: {:?}", &buf2);
    //
    // // Wrap is SOCKS5 send request
    // let socks5_send = nym_socks5_requests::request::Socks5Request::new_send(
    //     Socks5ProtocolVersion::Legacy,
    //     conn_id,
    //     buf2,
    //     false,
    // );
    //
    // let msg_send = Socks5ProviderRequest::new_provider_data(
    //     nym_socks5_client_core::config::Config::default()
    //         .get_socks5()
    //         .get_provider_interface_version(),
    //     socks5_send,
    // );
    //
    // client
    //     .send_bytes(
    //         client_address,
    //         msg_send.into_bytes(),
    //         IncludedSurbs::ExposeSelfAddress,
    //     )
    //     .await;
    //
    // println!("Waiting for message (ctrl-c to exit)");
    // client
    //     .on_messages(|msg| {
    //         if let Ok(res) = Socks5Response::try_from_bytes(msg.message.as_slice()) {
    //             println!("️✅  Socks5Response: {:?}", res);
    //             if let Socks5ResponseContent::NetworkData(data) = res.content {
    //                 // data.data is really an OrderedMessage
    //                 let response_ordered_message =
    //                     OrderedMessage::try_from_bytes(data.data).unwrap();
    //
    //                 println!(
    //                     "️🤖  Socks5ResponseContent::NetworkData: {}",
    //                     String::from_utf8_lossy(&response_ordered_message.data)
    //                 );
    //
    //                 if !response_ordered_message.data.is_empty() {
    //                     println!("️✅  resp: {:?}", response_ordered_message.data);
    //
    //                     // let response = http::response::Response::try_parse(resp).unwrap();
    //                     let mut decoder =
    //                         ResponseDecoder::<BodyDecoder<RemainingBytesDecoder>>::default();
    //                     let response = decoder
    //                         .decode_from_bytes(response_ordered_message.data.as_ref())
    //                         .unwrap();
    //
    //                     println!("➡️   decoded: {:?}", response);
    //                     println!("🚀  decoded: {}", String::from_utf8_lossy(response.body()));
    //                 }
    //             }
    //         }
    //
    //         // println!("️✅  Received: {}", String::from_utf8_lossy(&msg.message))
    //     })
    //     .await;
}
