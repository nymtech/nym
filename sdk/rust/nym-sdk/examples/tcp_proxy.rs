use nym_sdk::tcp_proxy;
use rand::Rng;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio_stream::StreamExt;
use tokio_util::codec;

// This is a basic example which opens a single TCP connection and streams between a client and
// server, so only uses a single session under the hood. See <MORE ELABORATE EXAMPLE> for use of
// multiple sessions/streams.
#[tokio::main]
async fn main() {
    // Comment this out to just see println! statements from this example.
    // Nym client logging is very informative but quite verbose.
    //
    // Run with RUST_LOG="debug" to see the Message Decay related logging if you want
    // a better idea of the internals of the proxy message ordering.
    nym_bin_common::logging::setup_logging();

    let upstream_tcp_addr = "127.0.0.1:9067";
    let mut proxy_server = tcp_proxy::NymProxyServer::new(upstream_tcp_addr, "~/tmp/server_client")
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

    // 'Server side' thread: just echoing back received messages.
    task::spawn(async move {
        let listener = TcpListener::bind(upstream_tcp_addr).await.unwrap();
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            let codec = codec::BytesCodec::new();
            let mut framed_read = codec::FramedRead::new(read, codec);
            while let Some(Ok(bytes)) = framed_read.next().await {
                // TODO make logging / parsing nicer
                println!("<< server received: {bytes:#?}");
                let reply = format!("reply to {:?}", bytes);
                write
                    .write_all(reply.as_bytes())
                    .await
                    .expect("couldnt send reply");
                println!(">> server sent {reply:?}");
            }
        }
    });

    // Just wait for Nym clients to connect, TCP clients to bind, etc.
    println!("waiting for everything to be set up..");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Now the client and server proxies are running we can create and pipe traffic to/from
    // a socket on the same port as our ProxyClient instance as if we were just communicating
    // between a client and host via a normal TcpStream - albeit with a decent amount of additional latency.
    //
    // The assumption regarding integration is that you know what you're sending, and will do proper
    // framing before and after, know what data types you're expecting, etc. Here we just pipe bytes
    // and use tokio's `Bytecodec` under the hood.
    let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let (read, mut write) = stream.into_split();

    task::spawn(async move {
        for i in 0..50 {
            let msg = format!("message {}", i);
            write
                .write_all(msg.as_bytes())
                .await
                .expect("couldn't write to stream");
            println!(">> client sent message {i} to server");
            let mut rng = rand::thread_rng();
            let delay: f64 = rng.gen_range(0.1..4.0);
            // Using std::sleep here as we do want to block the thread to somewhat emulate
            // IRL delays.
            std::thread::sleep(tokio::time::Duration::from_secs_f64(delay));
        }
    });

    let codec = codec::BytesCodec::new();
    let mut framed_read = codec::FramedRead::new(read, codec);
    while let Some(Ok(bytes)) = framed_read.next().await {
        println!("<< client received: {bytes:#?}");
    }

    tokio::signal::ctrl_c().await.unwrap();
}
