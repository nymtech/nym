use crate::client::received_buffer::ReceivedBufferRequestSender;
use crate::client::topology_control::TopologyAccessor;
use crate::client::InputMessageSender;
use crate::sockets::websocket::connection::{Connection, ConnectionData};
use log::*;
use sphinx::route::DestinationAddressBytes;
use std::net::SocketAddr;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use topology::NymTopology;

async fn process_socket_connection<T: NymTopology>(
    stream: tokio::net::TcpStream,
    connection_data: ConnectionData<T>,
) {
    match Connection::try_accept(stream, connection_data).await {
        None => warn!("Failed to establish websocket connection"),
        Some(mut conn) => conn.start_handling().await,
    }
}

pub(crate) fn run<T: NymTopology + 'static>(
    handle: &Handle,
    port: u16,
    msg_input: InputMessageSender,
    msg_query: ReceivedBufferRequestSender,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
) -> JoinHandle<()> {
    let handle_clone = handle.clone();
    handle.spawn(async move {
        let address = SocketAddr::new("127.0.0.1".parse().unwrap(), port);
        info!("Starting websocket listener at {:?}", address);
        let mut listener = tokio::net::TcpListener::bind(address).await.unwrap();
        let connection_data =
            ConnectionData::new(msg_input, msg_query, self_address, topology_accessor);

        // in theory there should only ever be a single connection made to the listener
        // but it's not significantly more difficult to allow more of them if needed
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            {
                let connection_data = connection_data.clone();
                handle_clone
                    .spawn(async move { process_socket_connection(stream, connection_data).await });
            }
        }
    })
}
