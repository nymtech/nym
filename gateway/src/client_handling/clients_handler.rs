use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use hmac::{Hmac, Mac};
use log::*;
use sha2::Sha256;
use tokio::task::JoinHandle;

use crypto::encryption;
use gateway_requests::auth_token::AuthToken;
use nymsphinx::DestinationAddressBytes;

use crate::client_handling::ledger::ClientLedger;
use crate::client_handling::websocket::message_receiver::MixMessageSender;
use crate::storage::ClientStorage;
use std::path::PathBuf;

pub(crate) type ClientsHandlerRequestSender = mpsc::UnboundedSender<ClientsHandlerRequest>;
pub(crate) type ClientsHandlerRequestReceiver = mpsc::UnboundedReceiver<ClientsHandlerRequest>;

pub(crate) type ClientsHandlerResponseSender = oneshot::Sender<ClientsHandlerResponse>;
pub(crate) type ClientsHandlerResponseReceiver = oneshot::Receiver<ClientsHandlerResponse>;

#[derive(Debug)]
pub(crate) enum ClientsHandlerRequest {
    // client
    Register(
        DestinationAddressBytes,
        MixMessageSender,
        ClientsHandlerResponseSender,
    ),
    Authenticate(
        DestinationAddressBytes,
        AuthToken,
        MixMessageSender,
        ClientsHandlerResponseSender,
    ),
    Disconnect(DestinationAddressBytes),

    // mix
    //    EmptyInbox(DestinationAddressBytes),
    IsOnline(DestinationAddressBytes, ClientsHandlerResponseSender),
}

#[derive(Debug)]
pub(crate) enum ClientsHandlerResponse {
    Register(AuthToken),
    Authenticate(bool),
    IsOnline(Option<MixMessageSender>),
    Error(Box<dyn std::error::Error + Send + Sync>),
}

pub(crate) struct ClientsHandler {
    secret_key: Arc<encryption::PrivateKey>,
    open_connections: HashMap<DestinationAddressBytes, MixMessageSender>,
    clients_ledger: ClientLedger,
    clients_inbox_storage: ClientStorage,
}

impl ClientsHandler {
    pub(crate) fn new(
        secret_key: Arc<encryption::PrivateKey>,
        ledger_path: PathBuf,
        clients_inbox_storage: ClientStorage,
    ) -> Self {
        ClientsHandler {
            secret_key,
            open_connections: HashMap::new(),
            clients_ledger: ClientLedger::load(ledger_path).unwrap(),
            clients_inbox_storage,
        }
    }

    fn make_error_response<E>(&self, err: E) -> ClientsHandlerResponse
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        ClientsHandlerResponse::Error(err.into())
    }

    // best effort sending error responses
    fn send_error_response<E>(&self, err: E, mut res_channel: ClientsHandlerResponseSender)
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        if let Err(_) = res_channel.send(self.make_error_response(err)) {
            error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
        }
    }

    fn generate_new_auth_token(&self, client_address: DestinationAddressBytes) -> AuthToken {
        type HmacSha256 = Hmac<Sha256>;

        // note that `new_varkey` doesn't even have an execution branch returning an error
        // (true as of hmac 0.7.1)
        let mut auth_token_raw = HmacSha256::new_varkey(&self.secret_key.to_bytes()).unwrap();
        auth_token_raw.input(client_address.as_bytes());
        let mut auth_token = [0u8; 32];
        auth_token.copy_from_slice(auth_token_raw.result().code().as_slice());
        AuthToken::from_bytes(auth_token)
    }

    async fn push_stored_messages_to_client_and_save_channel(
        &mut self,
        client_address: DestinationAddressBytes,
        comm_channel: MixMessageSender,
    ) {
        // TODO: it is possible that during a small window some of client messages will be "lost",
        // i.e. be stored on the disk rather than pushed to the client, reason for this is as follows:
        // now we push all stored messages from client's inbox to its websocket connection
        // however, say, at the same time there's new message to the client - it gets stored on the disk!
        // And only after this methods exits, mix receivers will become aware of the client
        // connection going online and being able to forward traffic there.
        //
        // possible solution: spawn a future to empty inbox in X seconds rather than immediately
        // JS: I will most likely do that (with including entries to config, etc.) once the
        // basic version is up and running as not to waste time on it now

        // NOTE: THIS IGNORES MESSAGE RETRIEVAL LIMIT AND TAKES EVERYTHING!
        let all_stored_messages = match self
            .clients_inbox_storage
            .retrieve_all_client_messages(client_address.clone())
            .await
        {
            Ok(msgs) => msgs,
            Err(e) => {
                error!(
                    "failed to retrieve client messages. {:?} inbox might be corrupted now - {:?}",
                    client_address.to_base58_string(),
                    e
                );
                return;
            }
        };

        let (messages, paths): (Vec<_>, Vec<_>) = all_stored_messages
            .into_iter()
            .map(|c| c.into_tuple())
            .unzip();

        if let Err(_) = comm_channel.unbounded_send(messages) {
            error!("Somehow we failed to stored messages to a fresh client channel - there seem to be a weird bug present!");
        } else {
            // but if all went well, we can now delete it
            if let Err(e) = self.clients_inbox_storage.delete_files(paths).await {
                error!(
                    "Failed to remove client ({:?}) files - {:?}",
                    client_address.to_base58_string(),
                    e
                );
            } else {
                // finally, everything was fine - we retrieved everything, we deleted everything,
                // we assume we can now safely delegate client message pushing
                self.open_connections.insert(client_address, comm_channel);
            }
        }
    }

    async fn handle_register_request(
        &mut self,
        address: DestinationAddressBytes,
        comm_channel: MixMessageSender,
        mut res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing register new client request: {:?}",
            address.to_base58_string()
        );

        if self.open_connections.get(&address).is_some() {
            warn!(
                "Tried to process register request for a client with an already opened connection!"
            );
            self.send_error_response("duplicate connection detected", res_channel);
            return;
        }

        // I presume some additional checks will go here:
        // ...
        let auth_token = self.generate_new_auth_token(address.clone());
        if self
            .clients_ledger
            .insert_token(auth_token.clone(), address.clone())
            .unwrap()
            .is_some()
        {
            info!(
                "Client {:?} was already registered before!",
                address.to_base58_string()
            )
        } else if let Err(e) = self
            .clients_inbox_storage
            .create_storage_dir(address.clone())
            .await
        {
            error!("We failed to create inbox directory for the client -{:?}\nReverting issued token...", e);
            // we must revert our changes if this operation failed
            self.clients_ledger.remove_token(&address).unwrap();
            self.send_error_response("failed to issue an auth token", res_channel);
            return;
        }

        self.push_stored_messages_to_client_and_save_channel(address, comm_channel)
            .await;

        if let Err(_) = res_channel.send(ClientsHandlerResponse::Register(auth_token)) {
            error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
        }
    }

    async fn handle_authenticate_request(
        &mut self,
        address: DestinationAddressBytes,
        token: AuthToken,
        comm_channel: MixMessageSender,
        mut res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing authenticate client request: {:?}",
            address.to_base58_string()
        );

        if self.open_connections.get(&address).is_some() {
            warn!("Tried to process authenticate request for a client with an already opened connection!");
            self.send_error_response("duplicate connection detected", res_channel);
            return;
        }

        if self.clients_ledger.verify_token(&token, &address).unwrap() {
            self.push_stored_messages_to_client_and_save_channel(address, comm_channel)
                .await;
            if let Err(_) = res_channel.send(ClientsHandlerResponse::Authenticate(true)) {
                error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
            }
        } else {
            if let Err(_) = res_channel.send(ClientsHandlerResponse::Authenticate(false)) {
                error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
            }
        }
    }

    fn handle_disconnect(&mut self, address: DestinationAddressBytes) {
        debug!(
            "Processing disconnect client request: {:?}",
            address.to_base58_string()
        );
        self.open_connections.remove(&address);
    }

    fn handle_is_online_request(
        &self,
        address: DestinationAddressBytes,
        mut res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing is online request for: {:?}",
            address.to_base58_string()
        );

        let response_value = self
            .open_connections
            .get(&address)
            .map(|channel| channel.clone());
        // if this fails, it's a critical failure, because mix handlers should ALWAYS be online
        res_channel
            .send(ClientsHandlerResponse::IsOnline(response_value))
            .unwrap();
    }

    pub(crate) async fn run(
        &mut self,
        mut request_receiver_channel: ClientsHandlerRequestReceiver,
    ) {
        while let Some(request) = request_receiver_channel.next().await {
            match request {
                ClientsHandlerRequest::Register(address, comm_channel, res_channel) => {
                    self.handle_register_request(address, comm_channel, res_channel)
                        .await
                }
                ClientsHandlerRequest::Authenticate(address, token, comm_channel, res_channel) => {
                    self.handle_authenticate_request(address, token, comm_channel, res_channel)
                        .await
                }
                ClientsHandlerRequest::Disconnect(address) => self.handle_disconnect(address),
                ClientsHandlerRequest::IsOnline(address, res_channel) => {
                    self.handle_is_online_request(address, res_channel)
                }
            };
        }
        error!("Something bad has happened and we stopped listening for requests!");
    }

    pub(crate) fn start(mut self) -> (JoinHandle<()>, ClientsHandlerRequestSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            tokio::spawn(async move { self.run(receiver).await }),
            sender,
        )
    }
}
