use bincode;
use nym_sdk::tcp_proxy;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::StreamExt;
use tokio_util::codec;
use tracing_subscriber;

#[derive(Serialize, Deserialize, Debug)]
struct ExampleMessage {
    message_id: i8,
    message_bytes: Vec<u8>,
}

// This is a basic example which opens a single TCP connection and writes a bunch of messages between a client and an echo
// server, so only uses a single session under the hood and doesn't really show off the message ordering capabilities; this is mainly
// just a quick introductory illustration on how:
// - the mixnet does message ordering
// - the NymProxyClient and NymProxyServer can be hooked into and used to communicate between two otherwise pretty vanilla TcpStreams
//
// For a more irl example checkout tcp_proxy_multistream.rs
#[tokio::main]
async fn main() {
    // Keep track of sent/received messages
    let counter = Arc::new(Mutex::new(0));

    // Comment this out to just see println! statements from this example, as Nym client logging is very informative but quite verbose.
    // The Message Decay related logging gives you an ideas of the internals of the proxy message ordering. To see the contents of the msg buffer, sphinx packet chunking, etc change the tracing::Level to DEBUG.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let upstream_tcp_addr = "127.0.0.1:9067";
    // This dir gets cleaned up at the end: NOTE if you switch env between tests without letting the file do the automatic cleanup, make sure to manually remove this directory up before running again, otherwise your client will attempt to use these keys for the new env
    let conf_path = "./tmp/nym-proxy-server-config";
    // Configure our clients to use the Canary test network: you can switch this to use any of the files in `../../../envs/`
    let env_path = "../../../envs/canary.env".to_string();

    let mut proxy_server =
        tcp_proxy::NymProxyServer::new(upstream_tcp_addr, conf_path, Some(env_path.clone()))
            .await
            .unwrap();
    let proxy_nym_addr = proxy_server.nym_address();

    // We'll run the instance with a long timeout since we're sending everything down the same Tcp connection, so should be using a single session.
    // Within the TcpProxyClient, individual client shutdown is triggered by the timeout.
    let proxy_client =
        tcp_proxy::NymProxyClient::new(*proxy_nym_addr, "127.0.0.1", "8080", 30, Some(env_path))
            .await
            .unwrap();

    task::spawn(async move {
        let _ = proxy_server.run_with_shutdown().await;
    });

    task::spawn(async move {
        let _ = proxy_client.run().await;
    });

    // 'Server side' thread: echo back incoming as response to the messages sent in the 'client side' thread below
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
                            "<< server received {}: {} bytes",
                            msg.message_id,
                            msg.message_bytes.len()
                        );
                        let msg = ExampleMessage {
                            message_id: msg.message_id,
                            message_bytes: msg.message_bytes,
                        };
                        let serialised = bincode::serialize(&msg).unwrap();
                        write
                            .write_all(&serialised)
                            .await
                            .expect("couldnt send reply");
                        println!(
                            ">> server sent {}: {} bytes",
                            msg.message_id,
                            msg.message_bytes.len()
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

    // Now the client and server proxies are running we can create and pipe traffic to/from
    // a socket on the same port as our ProxyClient instance as if we were just communicating
    // between a client and host via a normal TcpStream - albeit with a decent amount of additional latency.
    //
    // The assumption regarding integration is that you know what you're sending, and will do proper
    // framing before and after, know what data types you're expecting, etc; the proxies are just piping bytes
    // back and forth using tokio's `Bytecodec` under the hood.
    let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let (read, mut write) = stream.into_split();

    // 'Client side' thread; lets just send a bunch of messages to the server with variable delays between them, with an id to keep track of ordering in the printlns; the mixnet only guarantees message delivery, not ordering. You might not be necessarily streaming traffic in this manner IRL, but this example is a good illustration of how messages travel through the mixnet.
    // - On the level of individual messages broken into multiple packets, the Proxy abstraction deals with making sure that everything is sent between the sockets in the corrent order.
    // - On the level of different messages, this is not enforced: you might see in the logs that message 1 arrives at the server and is reconstructed after message 2.
    task::spawn(async move {
        let mut rng = SmallRng::from_entropy();
        for i in 0..10 {
            let random_bytes = gen_bytes_fixed(i as usize);
            let msg = ExampleMessage {
                message_id: i,
                message_bytes: random_bytes,
            };
            let serialised = bincode::serialize(&msg).unwrap();
            write
                .write_all(&serialised)
                .await
                .expect("couldn't write to stream");
            println!(">> client sent {}: {} bytes", &i, msg.message_bytes.len());
            let delay = rng.gen_range(3.0..7.0);
            tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay.clone())).await;
        }
    });

    let codec = codec::BytesCodec::new();
    let mut framed_read = codec::FramedRead::new(read, codec);
    while let Some(Ok(bytes)) = framed_read.next().await {
        match bincode::deserialize::<ExampleMessage>(&bytes) {
            Ok(msg) => {
                println!(
                    "<< client received {}: {} bytes",
                    msg.message_id,
                    msg.message_bytes.len()
                );
                let counter = Arc::clone(&counter);
                let mut newcount = counter.lock().await;
                *newcount += 1;
                println!(":: messages received back: {:?}/10", newcount);
            }
            Err(e) => {
                println!("<< client received something that wasn't an example message of {} bytes. error: {}", bytes.len(), e);
            }
        }
    }

    // Once timeout is passed, you can either wait for graceful shutdown or just hard stop it.
    signal::ctrl_c().await.unwrap();
    println!(":: CTRL+C received, shutting down + cleanup up proxy server config files");
    fs::remove_dir_all(conf_path).unwrap();
}

fn gen_bytes_fixed(i: usize) -> Vec<u8> {
    // let amounts = vec![1, 10, 50, 100, 150, 200, 350, 500, 750, 1000];
    let amounts = vec![158, 1088, 505, 1001, 150, 200, 3500, 500, 750, 100];
    let len = amounts[i];
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen::<u8>()).collect()
}