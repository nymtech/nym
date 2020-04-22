use crate::client_handling::ledger::ClientLedger;
use crate::client_handling::websocket::message_receiver::MixMessageSender;
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use log::*;
use nymsphinx::DestinationAddressBytes;
use std::collections::HashMap;
use tokio::task::JoinHandle;

type temp_AuthToken = String;

pub(crate) type ClientsHandlerRequestSender = mpsc::UnboundedSender<ClientsHandlerRequest>;
pub(crate) type ClientsHandlerRequestReceiver = mpsc::UnboundedReceiver<ClientsHandlerRequest>;

pub(crate) type ClientsHandlerResponseSender = oneshot::Sender<ClientsHandlerResponse>;
pub(crate) type ClientsHandlerResponseReceiver = oneshot::Receiver<ClientsHandlerResponse>;

pub(crate) enum ClientsHandlerRequest {
    // client
    Register(
        DestinationAddressBytes,
        MixMessageSender,
        ClientsHandlerResponseSender,
    ),
    Authenticate(
        temp_AuthToken,
        MixMessageSender,
        ClientsHandlerResponseSender,
    ),

    // mix
    IsOnline(DestinationAddressBytes, ClientsHandlerResponseSender),
}

pub(crate) enum ClientsHandlerResponse {
    Register(Option<temp_AuthToken>),
    Authenticate(bool),
    IsOnline(Option<MixMessageSender>),
}

pub(crate) struct ClientsHandler {
    open_connections: HashMap<DestinationAddressBytes, MixMessageSender>, //    clients_ledger: unimplemented!(),
    clients_ledger: ClientLedger,
    request_receiver_channel: ClientsHandlerRequestReceiver,
}

impl ClientsHandler {
    pub(crate) fn new(request_receiver_channel: ClientsHandlerRequestReceiver) -> Self {
        ClientsHandler {
            open_connections: HashMap::new(),
            request_receiver_channel,
            clients_ledger: unimplemented!(),
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(request) = self.request_receiver_channel.next().await {
            // handle request
        }
    }

    pub(crate) fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
