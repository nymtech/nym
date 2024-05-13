use axum::extract::State;
use axum::http::Response;
use axum::routing::get;
use axum::Router;
use futures::StreamExt;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sphinx::chunking::FRAGMENTS_RECEIVED;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::os::unix::thread;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::ClientWrapper;

pub struct HttpServer {
    listener: SocketAddr,
    cancel: CancellationToken,
}

#[derive(Clone)]
struct AppState {
    clients: Vec<Arc<RwLock<ClientWrapper>>>,
}

impl HttpServer {
    pub fn new(listener: SocketAddr, cancel: CancellationToken) -> Self {
        HttpServer { listener, cancel }
    }

    pub async fn run(self, clients: Vec<Arc<RwLock<ClientWrapper>>>) -> anyhow::Result<()> {
        let n_clients = clients.len();
        let state = AppState { clients };
        let app = Router::new().route("/", get(handler).with_state(state));
        let listener = tokio::net::TcpListener::bind(self.listener).await?;

        let shutdown_future = self.cancel.cancelled();
        let server_future = axum::serve(listener, app).into_future();

        println!("##########################################################################################");
        println!("######################### HTTP server running, with {} clients ############################################", n_clients);
        println!("##########################################################################################");

        tokio::select! {
            _ = shutdown_future => {
                println!("received shutdown");
            }
            res = server_future => {
                println!("the http server has terminated");
                if let Err(err) = res {
                    println!("with the following error: {err}");
                    return Err(err.into())
                }
            }
        }

        Ok(())
    }
}

async fn handler(State(state): State<AppState>) -> Response<String> {
    send_receive_mixnet(state).await
}

async fn send_receive_mixnet(state: AppState) -> Response<String> {
    let response = Response::builder();
    // let mut client = match make_client().await {
    //     Ok(client) => client,
    //     Err(e) => {
    //         return response
    //             .status(500)
    //             .body(format!("Failed to create mixnet client: {e}"))
    //             .unwrap();
    //     }
    // };

    let client = state.clients.choose(&mut rand::thread_rng()).unwrap();

    // Be able to get our client address
    let our_address = *client.read().await.client.nym_address();
    // println!("Our client nym address is: {our_address}");

    let sender = client.read().await.client.split_sender();

    let recv = Arc::clone(client);
    // receiving task
    let receiving_task_handle = tokio::spawn(async move {
        if let Some(received) = recv.write().await.client.next().await {
            println!("Received: {}", String::from_utf8_lossy(&received.message));
            println!("{:?}", *FRAGMENTS_RECEIVED);
        }

        // client.write().await.disconnect().await;
    });

    let mut rng = thread_rng();
    let msg = (0..32).map(|_| rng.gen::<char>()).collect::<String>();
    let sent_msg = msg.clone();

    let topology = client
        .read()
        .await
        .client
        .read_current_topology()
        .await
        .unwrap();

    // sending task
    let sending_task_handle = tokio::spawn(async move {
        match sender.send_plain_message(our_address, &msg).await {
            Ok(_) => println!("Sent message: {msg}"),
            Err(e) => println!("Failed to send message: {e}"),
        };
    });

    // wait for both tasks to be done
    println!("waiting for shutdown");
    match sending_task_handle.await {
        Ok(_) => {}
        Err(e) => {
            return response
                .status(500)
                .body(format!("Failed to send message: {e}"))
                .unwrap()
        }
    };
    match receiving_task_handle.await {
        Ok(_) => {}
        Err(e) => {
            return response
                .status(500)
                .body(format!("Failed to receive message: {e}"))
                .unwrap()
        }
    };
    response.status(200).body(sent_msg).unwrap()
}
