use crate::node::packet_processing::PacketProcessor;
use log::*;
use std::io;
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

async fn process_received_packet(
    packet_data: [u8; sphinx::PACKET_SIZE],
    processing_data: PacketProcessor,
) {
    // if we fail then we fail
    processing_data.process_sphinx_packet(packet_data);

    // let fwd_data = match PacketProcessor::process_sphinx_data_packet(
    //     &packet_data,
    //     processing_data,
    // )
    // .await
    // {
    //     Ok(fwd_data) => fwd_data,
    //     Err(e) => {
    //         warn!("failed to process sphinx packet: {:?}", e);
    //         return;
    //     }
    // };
    // PacketProcessor::wait_and_forward(fwd_data).await;
}

async fn process_socket_connection(
    mut socket: tokio::net::TcpStream,
    packet_processor: PacketProcessor,
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

                // we must be able to handle multiple packets from same connection independently
                tokio::spawn(process_received_packet(
                    buf.clone(),
                    // note: processing_data is relatively cheap (and safe) to clone -
                    // it contains arc to private key and metrics reporter (which is just
                    // a single mpsc unbounded sender)
                    // TODO: channel to tcp client
                    packet_processor.clone(),
                ))
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

pub(crate) fn run_socket_listener(
    handle: &Handle,
    addr: SocketAddr,
    packet_processor: PacketProcessor,
) -> JoinHandle<io::Result<()>> {
    let handle_clone = handle.clone();
    handle.spawn(async move {
        let mut listener = tokio::net::TcpListener::bind(addr).await?;
        loop {
            let (socket, _) = listener.accept().await?;

            let thread_packet_processor = packet_processor.clone();
            handle_clone.spawn(async move {
                process_socket_connection(socket, thread_packet_processor).await;
            });
        }
    })
}
