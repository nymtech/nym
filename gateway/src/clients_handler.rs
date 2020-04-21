use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use log::*;
use nymsphinx::DestinationAddressBytes;
use std::collections::HashMap;
use tokio::task::JoinHandle;

type temp_websocket_channel = mpsc::UnboundedSender<()>;
type temp_AuthToken = String;
type temp_ledger = String;

pub(crate) type ClientsHandlerRequestSsender = mpsc::UnboundedSender<ClientsHandlerRequest>;
pub(crate) type ClientsHandlerRequestReceiver = mpsc::UnboundedReceiver<ClientsHandlerRequest>;

pub(crate) type ClientsHandlerResponseSender = oneshot::Sender<ClientsHandlerResponse>;
pub(crate) type ClientsHandlerResponseReceiver = oneshot::Receiver<ClientsHandlerResponse>;

pub(crate) enum ClientsHandlerRequest {
    // client
    Register(
        DestinationAddressBytes,
        temp_websocket_channel,
        ClientsHandlerResponseSender,
    ),
    Authenticate(
        temp_AuthToken,
        temp_websocket_channel,
        ClientsHandlerResponseSender,
    ),

    // mix
    IsOnline(DestinationAddressBytes, ClientsHandlerResponseSender),
}

pub(crate) enum ClientsHandlerResponse {
    Register(Option<temp_AuthToken>),
    Authenticate(bool),
    IsOnline(Option<temp_websocket_channel>),
}

pub(crate) struct ClientsHandler {
    open_connections: HashMap<DestinationAddressBytes, temp_websocket_channel>, //    clients_ledger: unimplemented!(),
    clients_ledger: temp_ledger,
    request_receiver_channel: ClientsHandlerRequestReceiver,
}

impl ClientsHandler {
    pub(crate) fn new(request_receiver_channel: ClientsHandlerRequestReceiver) -> Self {
        ClientsHandler {
            open_connections: HashMap::new(),
            clients_ledger: "TEMPORARY".into(),
            request_receiver_channel,
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(request) = self.request_receiver_channel.next().await {
            // handle request
        }
    }

    pub(crate) fn start(&'static mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
