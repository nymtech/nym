use nym_sdk::mixnet::Recipient;
use nym_sdk::tcp_proxy;
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::signal;
use tokio_stream::StreamExt;
use tokio_util::codec;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Serialize, Deserialize, Debug)]
struct ExampleMessage {
    message_id: i8,
    message_bytes: Vec<u8>,
    tcp_conn: i8,
}

// This example just starts off a bunch of Tcp connections on a loop to a remote endpoint: in this case the TcpListener behind the NymProxyServer instance on the echo server found in `nym/tools/echo-server/`. It pipes a few messages to it, logs the replies, and keeps track of the number of replies received per connection.
//
// To run:
// - run the echo server with `cargo run`
// - run this example with `cargo run --example tcp_proxy_multistream -- <ECHO_SERVER_NYM_ADDRESS> <ENV_FILE_PATH> <CLIENT_PORT>` e.g.
// cargo run --example tcp_proxy_multistream -- DMHyxo8n6sKWHHTVvjRVDxDSMX8gYXRU1AQ6UpwsrWiB.6STYCWGWyRxqn2juWdgjMkAMsT9EaAzPpLWq5zkS68MB@CJG5zTcmoLijmDrtAiLV9PZHxNz8LQu6hmgA89V2RxxL ../../../envs/canary.env 8080
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_address = env::args().nth(1).expect("Server address not provided");
    let server: Recipient =
        Recipient::try_from_base58_string(&server_address).expect("Invalid server address");

    // Comment this out to just see println! statements from this example.
    // Nym client logging is very informative but quite verbose.
    // The Message Decay related logging gives you an ideas of the internals of the proxy message ordering: you need to switch
    // to DEBUG to see the contents of the msg buffer, sphinx packet chunking, etc.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::new("info")
                .add_directive("nym_sdk::client_pool=info".parse().unwrap())
                .add_directive("nym_sdk::tcp_proxy_client=debug".parse().unwrap()),
        )
        .init();

    let env_path = env::args().nth(2).expect("Env file not specified");
    let env = env_path.to_string();

    let listen_port = env::args().nth(3).expect("Port not specified");

    // Within the TcpProxyClient, individual client shutdown is triggered by the timeout. The final argument is how many clients to keep in reserve in the client pool when running the TcpProxy.
    let proxy_client =
        tcp_proxy::NymProxyClient::new(server, "127.0.0.1", &listen_port, 80, Some(env), 3).await?;

    // For our disconnect() logic below
    let proxy_clone = proxy_client.clone();

    tokio::spawn(async move {
        proxy_client.run().await?;
        Ok::<(), anyhow::Error>(())
    });

    let example_cancel_token = CancellationToken::new();
    let client_cancel_token = example_cancel_token.clone();
    let watcher_cancel_token = example_cancel_token.clone();

    // Cancel listener thread
    tokio::spawn(async move {
        signal::ctrl_c().await?;
        println!(":: CTRL_C received, shutting down + cleanup up proxy server config files");
        watcher_cancel_token.cancel();
        proxy_clone.disconnect().await;
        Ok::<(), anyhow::Error>(())
    });

    println!("waiting for everything to be set up..");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    println!("done. sending bytes");

    // In the info traces you will see the different session IDs being set up, one for each TcpStream.
    for i in 0..4 {
        let client_cancel_inner_token = client_cancel_token.clone();
        if client_cancel_token.is_cancelled() {
            break;
        }
        let conn_id = i;
        let local_tcp_addr = format!("127.0.0.1:{}", listen_port.clone());
        tokio::spawn(async move {
            // Now the client and server proxies are running we can create and pipe traffic to/from
            // a socket on the same port as our ProxyClient instance as if we were just communicating
            // between a client and host via a normal TcpStream - albeit with a decent amount of additional latency.
            //
            // The assumption regarding integration is that you know what you're sending, and will do proper
            // framing before and after, know what data types you're expecting; the proxies are just piping bytes
            // back and forth using tokio's `Bytecodec` under the hood.

            let stream = TcpStream::connect(local_tcp_addr).await?;
            let (read, mut write) = stream.into_split();

            // Lets just send a bunch of messages to the server with variable delays between them, with a message and tcp connection ids to keep track of ordering on the server side (for illustrative purposes **only**; keeping track of anonymous replies is handled by the proxy under the hood with Single Use Reply Blocks (SURBs); for this illustration we want some kind of app-level message id, but irl most of the time you'll probably be parsing on e.g. the incoming response type instead)
            tokio::spawn(async move {
                for i in 0..4 {
                    if client_cancel_inner_token.is_cancelled() {
                        break;
                    }
                    let mut rng = SmallRng::from_entropy();
                    let delay: f64 = rng.gen_range(2.5..5.0);
                    tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay)).await;
                    let random_bytes = gen_bytes_fixed(i as usize);
                    let msg = ExampleMessage {
                        message_id: i,
                        message_bytes: random_bytes,
                        tcp_conn: conn_id,
                    };
                    let serialised = bincode::serialize(&msg)?;
                    write
                        .write_all(&serialised)
                        .await
                        .expect("couldn't write to stream");
                    println!(">> client sent msg {} on conn {}", &i, &conn_id);
                }
                Ok::<(), anyhow::Error>(())
            });

            tokio::spawn(async move {
                let mut reply_counter = 0;
                let codec = codec::BytesCodec::new();
                let mut framed_read = codec::FramedRead::new(read, codec);
                while let Some(Ok(bytes)) = framed_read.next().await {
                    match bincode::deserialize::<ExampleMessage>(&bytes) {
                        Ok(msg) => {
                            reply_counter += 1;
                            println!("<< conn {} received {}/4", msg.tcp_conn, reply_counter);
                        }
                        Err(e) => {
                            println!("<< client received something that wasn't an example message of {} bytes. error: {}", bytes.len(), e);
                        }
                    }
                }
            });
            Ok::<(), anyhow::Error>(())
        });
        let mut rng = SmallRng::from_entropy();
        let delay: f64 = rng.gen_range(4.5..7.0);
        tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay)).await;
    }

    loop {
        if example_cancel_token.is_cancelled() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    Ok(())
}

// emulate a series of small messages followed by a closing larger one
fn gen_bytes_fixed(i: usize) -> Vec<u8> {
    let amounts = [10, 15, 50, 1000, 2000];
    let len = amounts[i];
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen::<u8>()).collect()
}
