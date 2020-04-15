use crate::provider::client_handling::request_processing::{
    ClientProcessingResult, RequestProcessor,
};
use log::*;
use sfw_provider_requests::requests::{
    async_io::TokioAsyncRequestReader, ProviderRequest, ProviderRequestError,
};
use sfw_provider_requests::responses::{
    async_io::TokioAsyncResponseWriter, FailureResponse, ProviderResponse, PullResponse,
    RegisterResponse,
};
use std::io;
use std::net::SocketAddr;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

async fn process_request<'a>(
    response_writer: &mut TokioAsyncResponseWriter<'a, tokio::net::tcp::WriteHalf<'a>>,
    request: ProviderRequest,
    request_processor: &mut RequestProcessor,
) {
    match request_processor.process_client_request(request).await {
        Err(e) => {
            warn!("We failed to process client request - {:?}", e);
            let failure_response =
                ProviderResponse::Failure(FailureResponse::new(format!("{:?}", e)));
            if let Err(e) = response_writer.try_write_response(failure_response).await {
                debug!("Failed to write response to the client - {:?}", e);
            }
        }
        Ok(res) => match res {
            ClientProcessingResult::RegisterResponse(auth_token) => {
                let response = ProviderResponse::Register(RegisterResponse::new(auth_token));
                if let Err(e) = response_writer.try_write_response(response).await {
                    debug!("Failed to write response to the client - {:?}", e);
                }
            }
            ClientProcessingResult::PullResponse(retrieved_messages) => {
                let (messages, paths): (Vec<_>, Vec<_>) = retrieved_messages
                    .into_iter()
                    .map(|c| c.into_tuple())
                    .unzip();
                let response = ProviderResponse::Pull(PullResponse::new(messages));
                if response_writer.try_write_response(response).await.is_ok() {
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
    let peer_addr = socket.peer_addr();
    let (mut socket_reader, mut socket_writer) = socket.split();
    // TODO: benchmark and determine if below should be done:
    //        let mut socket_writer = tokio::io::BufWriter::new(socket_writer);
    //        let mut socket_reader = tokio::io::BufReader::new(socket_reader);

    let mut request_reader =
        TokioAsyncRequestReader::new(&mut socket_reader, request_processor.max_request_size());
    let mut response_writer = TokioAsyncResponseWriter::new(&mut socket_writer);

    loop {
        match request_reader.try_read_request().await {
            Err(ProviderRequestError::RemoteConnectionClosed) => {
                trace!("Remote connection closed.");
                return;
            }
            Err(ProviderRequestError::IOError(e)) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    debug!("Read buffer was not fully filled. Most likely the client ({:?}) closed the connection.\
                    Also closing the connection on this end.", peer_addr)
                } else {
                    warn!(
                        "failed to read from socket (source: {:?}). Closing the connection; err = {:?}",
                        peer_addr,
                        e
                    );
                }
                return;
            }
            Err(e) => {
                // let's leave it like this for time being and see if we need to decrease
                // logging level and / or close the connection
                warn!("the received request was invalid - {:?}", e);
                // should the connection be closed here? invalid request might imply
                // the subsequent requests in the reader buffer might not be aligned anymore
                // however, that might not necessarily be the case
                return;
            }
            // in here we do not really want to process multiple requests from the same
            // client concurrently as then we might end up with really weird race conditions
            // plus realistically it wouldn't really introduce any speed up
            Ok(request) => {
                process_request(&mut response_writer, request, &mut request_processor).await
            }
        }
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
