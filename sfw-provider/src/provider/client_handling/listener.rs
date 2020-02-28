use crate::provider::client_handling::request_processing::RequestProcessor;
use log::*;
use std::io;
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

async fn process_socket_connection(
    mut socket: tokio::net::TcpStream,
    request_processor: RequestProcessor,
) {
    let mut buf = [0u8; sphinx::PACKET_SIZE];
    loop {
        match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                trace!("Remote connection closed.");
                return;
            }
            Ok(n) => {
                if n != sphinx::PACKET_SIZE {
                    warn!("read data of different length than expected sphinx packet size - {} (expected {})", n, sphinx::PACKET_SIZE);
                    continue;
                }

                tokio::spawn(async {});

                // // we must be able to handle multiple packets from same connection independently
                // tokio::spawn(process_received_packet(
                //     buf.clone(),
                //     // TODO:
                //     // TODO:
                //     // note: processing_data is relatively cheap (and safe) to clone -
                //     // it contains arc to private key and metrics reporter (which is just
                //     // a single mpsc unbounded sender)
                //     request_processor.clone(),
                // ))
            }
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

// // TODO: FIGURE OUT HOW TO SET READ_DEADLINES IN TOKIO
// async fn process_client_socket_connection(
//     mut socket: tokio::net::TcpStream,
//     processing_data: Arc<ClientProcessingData>,
// ) {
//     let mut buf = [0; 1024];
//
//     // TODO: restore the for loop once we go back to persistent tcp socket connection
//     let response = match socket.read(&mut buf).await {
//         // socket closed
//         Ok(n) if n == 0 => {
//             trace!("Remote connection closed.");
//             Err(())
//         }
//         Ok(n) => {
//             match ClientRequestProcessor::process_client_request(
//                 buf[..n].as_ref(),
//                 processing_data,
//             )
//                 .await
//             {
//                 Err(e) => {
//                     warn!("failed to process client request; err = {:?}", e);
//                     Err(())
//                 }
//                 Ok(res) => Ok(res),
//             }
//         }
//         Err(e) => {
//             warn!("failed to read from socket; err = {:?}", e);
//             Err(())
//         }
//     };
//
//     if let Err(e) = socket.shutdown(Shutdown::Read) {
//         warn!("failed to close read part of the socket; err = {:?}", e)
//     }
//
//     match response {
//         Ok(res) => {
//             ServiceProvider::send_response(socket, &res).await;
//         }
//         _ => {
//             ServiceProvider::send_response(socket, b"bad foomp").await;
//         }
//     }
// }

// async fn send_response(mut socket: tokio::net::TcpStream, data: &[u8]) {
//     if let Err(e) = socket.write_all(data).await {
//         warn!("failed to write reply to socket; err = {:?}", e)
//     }
//     if let Err(e) = socket.shutdown(Shutdown::Write) {
//         warn!("failed to close write part of the socket; err = {:?}", e)
//     }
// }

// async fn start_client_listening(
//     address: SocketAddr,
//     store_dir: PathBuf,
//     client_ledger: Arc<Mutex<ClientLedger>>,
//     secret_key: encryption::PrivateKey,
//     message_retrieval_limit: u16,
// ) -> Result<(), ProviderError> {
//     let mut listener = tokio::net::TcpListener::bind(address).await?;
//     let processing_data = ClientProcessingData::new(
//         store_dir,
//         client_ledger,
//         secret_key,
//         message_retrieval_limit,
//     )
//         .add_arc();
//
//     loop {
//         let (socket, _) = listener.accept().await?;
//         // do note that the underlying data is NOT copied here; arc is incremented and lock is shared
//         // (if I understand it all correctly)
//         let thread_processing_data = processing_data.clone();
//         tokio::spawn(async move {
//             ServiceProvider::process_client_socket_connection(socket, thread_processing_data)
//                 .await
//         });
//     }
// }

// let client_future = rt.spawn(ServiceProvider::start_client_listening(
//     self.config.get_clients_listening_address(),
//     self.config.get_clients_inboxes_dir(),
//     thread_shareable_ledger,
//     self.sphinx_keypair.private_key().clone(), // CLONE IS DONE TEMPORARILY UNTIL PROVIDER IS REFACTORED THE MIXNODE STYLE
//     self.config.get_message_retrieval_limit(),
// ));

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
