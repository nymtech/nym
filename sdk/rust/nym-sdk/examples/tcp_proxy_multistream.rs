use bincode;
use nym_sdk::tcp_proxy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::task;
use tokio_stream::StreamExt;
use tokio_util::codec;

#[derive(Serialize, Deserialize, Debug)]
struct ExampleMessage {
    message_id: i8,
    message_bytes: Vec<u8>,
    tcp_conn: i8,
}

// This example emulates something like a streaming client opening a bunch of different TcpStreams to a remote server.
#[tokio::main]
async fn main() {
    // Nym client logging is very informative but quite verbose.
    // The Message Decay related logging gives you an ideas of the internals of the proxy message ordering.
    // Run with RUST_LOG="debug" to see this.
    // nym_bin_common::logging::setup_logging();

    let upstream_tcp_addr = "127.0.0.1:9067";
    // This dir gets cleaned up at the end
    let conf_path = "./tmp/nym-proxy-server-config-multi";
    let mut proxy_server = tcp_proxy::NymProxyServer::new(upstream_tcp_addr, conf_path)
        .await
        .unwrap();
    let proxy_nym_addr = proxy_server.nym_address();

    // Within the TcpProxyClient, individual client shutdown is triggered by the timeout.
    let proxy_client = tcp_proxy::NymProxyClient::new(*proxy_nym_addr, "127.0.0.1", "8080", 180)
        .await
        .unwrap();

    task::spawn(async move {
        let _ = proxy_server.run_with_shutdown().await;
    });

    task::spawn(async move {
        let _ = proxy_client.run().await;
    });

    // 'Server side' thread: echo back the messages sent in the 'client side' thread below, retain the message and conn ids for illustrative purposes
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
                        println!(
                            "<< server received {}: {} bytes on tcp conn {}",
                            msg.message_id,
                            msg.message_bytes.len(),
                            msg.tcp_conn
                        );
                        let reply = ExampleMessage {
                            message_id: msg.message_id,
                            message_bytes: msg.message_bytes.clone(),
                            tcp_conn: msg.tcp_conn,
                        };
                        let serialised = bincode::serialize(&reply).unwrap();
                        write
                            .write_all(&serialised)
                            .await
                            .expect("couldnt send reply");
                        println!(
                            ">> server sent {}: {} bytes on conn {}",
                            msg.message_id,
                            msg.message_bytes.len(),
                            msg.tcp_conn
                        );
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
    println!("done. sending bytes");

    // In the logging you will see the different session IDs being set up, one for each TcpStream.
    for i in 0..4 {
        let conn_id = i;
        println!("Starting TCP connection {}", conn_id);
        tokio::spawn(async move {
            // Now the client and server proxies are running we can create and pipe traffic to/from
            // a socket on the same port as our ProxyClient instance as if we were just communicating
            // between a client and host via a normal TcpStream - albeit with a decent amount of additional latency.
            //
            // The assumption regarding integration is that you know what you're sending, and will do proper
            // framing before and after, know what data types you're expecting; the proxies are just piping bytes
            // back and forth using tokio's `Bytecodec` under the hood.
            let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            let (read, mut write) = stream.into_split();

            // 'Client side' thread; lets just send a bunch of messages to the server with variable delays between them, with a message and tcp connection ids to keep track of ordering on the server side (for illustrative purposes **only**; keeping track of replying is handled by the proxy under the hood with SURBs)
            task::spawn(async move {
                for i in 0..4 {
                    let random_bytes = gen_bytes_fixed(i as usize);
                    let msg = ExampleMessage {
                        message_id: i,
                        message_bytes: random_bytes,
                        tcp_conn: conn_id,
                    };
                    let serialised = bincode::serialize(&msg).unwrap();
                    write
                        .write_all(&serialised)
                        .await
                        .expect("couldn't write to stream");
                    println!(
                        ">> client sent {}: {} bytes on conn {}",
                        &i,
                        msg.message_bytes.len(),
                        &conn_id
                    );
                    let mut rng = rand::thread_rng();
                    let delay: f64 = rng.gen_range(0.5..4.0);
                    // Using std::sleep here as we do want to block the thread to somewhat emulate IRL delays.
                    std::thread::sleep(tokio::time::Duration::from_secs_f64(delay));
                }
            });

            task::spawn(async move {
                let codec = codec::BytesCodec::new();
                let mut framed_read = codec::FramedRead::new(read, codec);
                while let Some(Ok(bytes)) = framed_read.next().await {
                    match bincode::deserialize::<ExampleMessage>(&bytes) {
                        Ok(msg) => {
                            println!(
                                "<< client received {}: {} bytes on conn {}",
                                msg.message_id,
                                msg.message_bytes.len(),
                                msg.tcp_conn
                            );
                        }
                        Err(e) => {
                            println!("<< client received something that wasn't an example message of {} bytes. error: {}", bytes.len(), e);
                        }
                    }
                }
            });
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
    }

    // Once timeout is passed, you can either wait for graceful shutdown or just hard stop it.
    signal::ctrl_c().await.unwrap();
    println!("CTRL+C received, shutting down + cleanup up proxy server config files");
    fs::remove_dir_all(conf_path).unwrap();
}

// emulate a series of small messages followed by a closing larger one
fn gen_bytes_fixed(i: usize) -> Vec<u8> {
    let amounts = vec![10, 15, 50, 1000];
    let len = amounts[i];
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen::<u8>()).collect()
}
