use crate::client_handling::clients_handler::ClientsHandlerRequestSender;
use log::*;
use tokio::{prelude::*, stream::StreamExt};
use tokio_tungstenite::{
    tungstenite::{protocol::Message, Error as WsError},
    WebSocketStream,
};

enum SocketStream<S: AsyncRead + AsyncWrite + Unpin> {
    RawTCP(S),
    UpgradedWebSocket(WebSocketStream<S>),
    Invalid,
}

pub(crate) struct Handle<S: AsyncRead + AsyncWrite + Unpin> {
    socket_connection: SocketStream<S>,
    clients_handler_sender: ClientsHandlerRequestSender,
}

impl<S> Handle<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(conn: S, clients_handler_sender: ClientsHandlerRequestSender) -> Self {
        Handle {
            socket_connection: SocketStream::RawTCP(conn),
            clients_handler_sender,
        }
    }

    async fn perform_websocket_handshake(&mut self) {
        self.socket_connection =
            match std::mem::replace(&mut self.socket_connection, SocketStream::Invalid) {
                SocketStream::RawTCP(conn) => {
                    // TODO: perhaps in the future, rather than panic here (and uncleanly shut tcp stream)
                    // return a result with an error?
                    let ws_stream = tokio_tungstenite::accept_async(conn)
                        .await
                        .expect("Failed to perform websocket handshake");
                    SocketStream::UpgradedWebSocket(ws_stream)
                }
                other => other,
            }
    }

    async fn next_websocket_request(&mut self) -> Option<Result<Message, WsError>> {
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.next().await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    async fn listen_for_requests(&mut self) {
        trace!("Started listening for incoming requests...");

        while let Some(msg) = self.next_websocket_request().await {
            // start handling here
            //        let msg = msg?;
            //        if msg.is_binary() {
            //            mixnet_client::forward_to_mixnode(msg.into_data(), Arc::clone(&client_ref)).await;
            //        }
        }

        trace!("The stream was closed!");
    }

    pub(crate) async fn start_handling(&mut self) {
        self.perform_websocket_handshake().await;
        trace!("Managed to perform websocket handshake!");
        self.listen_for_requests().await;
    }
}
