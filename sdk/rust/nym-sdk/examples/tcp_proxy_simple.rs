use nym_sdk::tcp_proxy;
use std::fs;
use bincode;
use rand::Rng;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio_stream::StreamExt;
// use tokio_util::sync::CancellationToken; // TODO introduce this again
use tokio_util::codec;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct ExampleMessage {
    message_id: i8,
    message_bytes: Vec<u8>
}

// This is a basic example which opens a single TCP connection and writes a bunch of messages between a client and
// server, so only uses a single session under the hood and doesn't really show off the message ordering capabilities.
// See tcp_proxy_multistream for use of multiple sessions/streams.
#[tokio::main]
async fn main() {
    // Comment this out to just see println! statements from this example.
    // Nym client logging is very informative but quite verbose.
    //
    // If instead you want to increase the verbosity, run with RUST_LOG="debug" to see the Message Decay
    // related logging if you want a better idea of the internals of the proxy message ordering.
    nym_bin_common::logging::setup_logging();

    let upstream_tcp_addr = "127.0.0.1:9067";
    // This dir gets cleaned up at the end
    let conf_path = "./tmp/nym-proxy-server-config";
    let mut proxy_server = tcp_proxy::NymProxyServer::new(upstream_tcp_addr, conf_path)
        .await
        .unwrap();
    let proxy_nym_addr = proxy_server.nym_address();
    let proxy_client = tcp_proxy::NymProxyClient::new(*proxy_nym_addr, "127.0.0.1", "8080", 60)
        .await
        .unwrap();

    task::spawn(async move {
        let _ = proxy_server.run_with_shutdown().await;
    });

    task::spawn(async move {
        let _ = proxy_client.run().await;
    });

    // 'Server side' thread: send back a bunch of random bytes as a response, retain the message id
    task::spawn(async move {
        let listener = TcpListener::bind(upstream_tcp_addr).await.unwrap();
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            let codec = codec::BytesCodec::new();
            let mut framed_read = codec::FramedRead::new(read, codec);
            while let Some(Ok(bytes)) = framed_read.next().await {
                match bincode::deserialize::<ExampleMessage>(&bytes) {
                    Ok(msg) => {
                        println!("<< server received msg {}: {} bytes", msg.message_id, msg.message_bytes.len());
                        let random_bytes = gen_bytes();
                        let msg = ExampleMessage {
                            message_id: msg.message_id,
                            message_bytes: random_bytes
                        };
                        let serialised = bincode::serialize(&msg).unwrap();
                        write
                            .write_all(&serialised)
                            .await
                            .expect("couldnt send reply");
                        println!(">> server sent reply {}: {} bytes", msg.message_id, msg.message_bytes.len());
                    }
                    Err(e) => {
                        println!("<< server received something that wasn't an example message of {} bytes. error: {}", bytes.len(), e);
                    }
                }
            }
        }
    });

    // Just wait for Nym clients to connect, TCP clients to bind, etc.
    println!("waiting for everything to be set up..");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    println!("done. sending random bytes with random(ish) delay");

    // Now the client and server proxies are running we can create and pipe traffic to/from
    // a socket on the same port as our ProxyClient instance as if we were just communicating
    // between a client and host via a normal TcpStream - albeit with a decent amount of additional latency.
    //
    // The assumption regarding integration is that you know what you're sending, and will do proper
    // framing before and after, know what data types you're expecting, etc. Here we just pipe bytes
    // and use tokio's `Bytecodec` under the hood.
    let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let (read, mut write) = stream.into_split();

    // Lets just send a bunch of messages to the server with variable delays between them, with an id to keep track of ordering on the server side, and random amounts of bytes
    task::spawn(async move {
        for i in 0..10 {
            let random_bytes = gen_bytes();
            let msg = ExampleMessage {
               message_id: i,
               message_bytes: random_bytes
            };
            let serialised = bincode::serialize(&msg).unwrap();
            write
                .write_all(&serialised)
                .await
                .expect("couldn't write to stream");
            println!(">> client sent msg {}: {} bytes", &i, msg.message_bytes.len());
            let mut rng = rand::thread_rng();
            let delay: f64 = rng.gen_range(1.0..4.0);
            // Using std::sleep here as we do want to block the thread to somewhat emulate
            // IRL delays.
            std::thread::sleep(tokio::time::Duration::from_secs_f64(delay));
        }
    });

    let codec = codec::BytesCodec::new();
    let mut framed_read = codec::FramedRead::new(read, codec);
    while let Some(Ok(bytes)) = framed_read.next().await {
        match bincode::deserialize::<ExampleMessage>(&bytes) {
            Ok(msg) => {
                println!("<< client received reply {}: {} bytes", msg.message_id, msg.message_bytes.len());
            }
            Err(e) => {
              println!("<< client received something that wasn't an example message of {} bytes. error: {}", bytes.len(), e);
            }
        }
    }

    println!("TODO add cancellation token + call here");
    tokio::time::sleep(tokio::time::Duration::from_secs(180)).await;
    fs::remove_dir_all(conf_path).unwrap();
}

fn gen_bytes() -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let len = rng.gen_range(10..=2000);
    (0..len).map(|_| rng.gen::<u8>()).collect()
}
