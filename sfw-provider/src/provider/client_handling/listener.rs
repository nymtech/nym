use crate::provider::client_handling::request_processing::{
    ClientProcessingResult, RequestProcessor,
};
use log::*;
use sfw_provider_requests::responses::{
    ErrorResponse, ProviderResponse, PullResponse, RegisterResponse,
};
use std::io;
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

async fn process_request(
    socket: &mut tokio::net::TcpStream,
    packet_data: &[u8],
    request_processor: &mut RequestProcessor,
) {
    match request_processor.process_client_request(packet_data).await {
        Err(e) => {
            warn!("We failed to process client request - {:?}", e);
            let response_bytes = ErrorResponse::new(format!("{:?}", e)).to_bytes();
            if let Err(e) = socket.write_all(&response_bytes).await {
                debug!("Failed to write response to the client - {:?}", e);
            }
        }
        Ok(res) => match res {
            ClientProcessingResult::RegisterResponse(auth_token) => {
                let response_bytes = RegisterResponse::new(auth_token).to_bytes();
                if let Err(e) = socket.write_all(&response_bytes).await {
                    debug!("Failed to write response to the client - {:?}", e);
                }
            }
            ClientProcessingResult::PullResponse(retrieved_messages) => {
                let (messages, paths): (Vec<_>, Vec<_>) = retrieved_messages
                    .into_iter()
                    .map(|c| c.into_tuple())
                    .unzip();
                let response_bytes = PullResponse::new(messages).to_bytes();
                if socket.write_all(&response_bytes).await.is_ok() {
                    // only delete stored messages if we managed to actually send the response
                    if let Err(e) = request_processor.delete_sent_messages(paths).await {
                        error!("Somehow failed to delete stored messages! - {:?}", e);
                    }
                }
            }
        },
    }
}

async fn process_socket_connection(
    mut socket: tokio::net::TcpStream,
    mut request_processor: RequestProcessor,
) {
    let mut buf = [0u8; 1024];
    loop {
        match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                trace!("Remote connection closed.");
                return;
            }
            // in here we do not really want to process multiple requests from the same
            // client concurrently as then we might end up with really weird race conditions
            // plus realistically it wouldn't really introduce any speed up
            Ok(n) => process_request(&mut socket, &buf[0..n], &mut request_processor).await,
            Err(e) => {
                warn!(
                    "failed to read from socket. Closing the connection; err = {:?}",
                    e
                );
                return;
            }
        };
    }
}

pub(crate) fn run_client_socket_listener(
    handle: &Handle,
    addr: SocketAddr,
    request_processor: RequestProcessor,
) -> JoinHandle<io::Result<()>> {
    let handle_clone = handle.clone();
    handle.spawn(async move {
        let mut listener = tokio::net::TcpListener::bind(addr).await?;
        loop {
            let (socket, _) = listener.accept().await?;

            let thread_request_processor = request_processor.clone();
            handle_clone.spawn(async move {
                process_socket_connection(socket, thread_request_processor).await;
            });
        }
    })
}
