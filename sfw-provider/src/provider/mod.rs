use crate::config::persistence::pathfinder::ProviderPathfinder;
use crate::config::Config;
use crate::provider::client_handling::{ClientProcessingData, ClientRequestProcessor};
use crate::provider::mix_handling::{MixPacketProcessor, MixProcessingData};
use crate::provider::storage::ClientStorage;
use crypto::encryption;
use directory_client::presence::providers::MixProviderClient;
use futures::io::Error;
use futures::lock::Mutex as FMutex;
use log::*;
use pemstore::pemstore::PemStore;
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use std::collections::HashMap;
use std::net::{Shutdown, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::prelude::*;
use tokio::runtime::Runtime;

mod client_handling;
mod mix_handling;
pub mod presence;
mod storage;

#[derive(Debug)]
pub enum ProviderError {
    TcpListenerBindingError,
    TcpListenerConnectionError,
    TcpListenerUnexpectedEof,

    TcpListenerUnknownError,
}

impl From<io::Error> for ProviderError {
    fn from(err: Error) -> Self {
        use ProviderError::*;
        match err.kind() {
            io::ErrorKind::ConnectionRefused => TcpListenerConnectionError,
            io::ErrorKind::ConnectionReset => TcpListenerConnectionError,
            io::ErrorKind::ConnectionAborted => TcpListenerConnectionError,
            io::ErrorKind::NotConnected => TcpListenerConnectionError,

            io::ErrorKind::AddrInUse => TcpListenerBindingError,
            io::ErrorKind::AddrNotAvailable => TcpListenerBindingError,
            io::ErrorKind::UnexpectedEof => TcpListenerUnexpectedEof,
            _ => TcpListenerUnknownError,
        }
    }
}

#[derive(Debug)]
pub struct ClientLedger(HashMap<AuthToken, DestinationAddressBytes>);

impl ClientLedger {
    fn new() -> Self {
        ClientLedger(HashMap::new())
    }

    fn add_arc_futures_mutex(self) -> Arc<FMutex<Self>> {
        Arc::new(FMutex::new(self))
    }

    fn has_token(&self, auth_token: &AuthToken) -> bool {
        self.0.contains_key(auth_token)
    }

    fn insert_token(
        &mut self,
        auth_token: AuthToken,
        client_address: DestinationAddressBytes,
    ) -> Option<DestinationAddressBytes> {
        self.0.insert(auth_token, client_address)
    }

    fn current_clients(&self) -> Vec<MixProviderClient> {
        self.0
            .iter()
            .map(|(_, v)| bs58::encode(v).into_string())
            .map(|pub_key| MixProviderClient { pub_key })
            .collect()
    }

    #[allow(dead_code)]
    fn load(_file: PathBuf) -> Self {
        unimplemented!()
    }
}

pub struct ServiceProvider {
    config: Config,
    sphinx_keypair: encryption::KeyPair,
    registered_clients_ledger: ClientLedger,
}

impl ServiceProvider {
    pub fn new(config: Config) -> Self {
        let sphinx_keypair = Self::load_sphinx_keys(&config);

        ServiceProvider {
            config,
            sphinx_keypair,
            // TODO: load initial ledger from file
            registered_clients_ledger: ClientLedger::new(),
        }
    }

    fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
        let sphinx_keypair = PemStore::new(ProviderPathfinder::new_from_config(&config_file))
            .read_encryption()
            .expect("Failed to read stored sphinx key files");
        println!(
            "Public encryption key: {}\nFor time being, it is identical to identity keys",
            sphinx_keypair.public_key().to_base58_string()
        );
        sphinx_keypair
    }

    async fn process_mixnet_socket_connection(
        mut socket: tokio::net::TcpStream,
        processing_data: Arc<RwLock<MixProcessingData>>,
    ) {
        let mut buf = [0u8; sphinx::PACKET_SIZE];

        // In a loop, read data from the socket and write the data back.
        loop {
            match socket.read(&mut buf).await {
                // socket closed
                Ok(n) if n == 0 => {
                    trace!("Remote connection closed.");
                    return;
                }
                Ok(_) => {
                    let store_data = match MixPacketProcessor::process_sphinx_data_packet(
                        buf.as_ref(),
                        processing_data.as_ref(),
                    ) {
                        Ok(sd) => sd,
                        Err(e) => {
                            warn!("failed to process sphinx packet; err = {:?}", e);
                            return;
                        }
                    };
                    let processing_data_lock = match processing_data.read() {
                        Ok(guard) => guard,
                        Err(e) => {
                            error!("processing data lock was poisoned! - {:?}", e);
                            std::process::exit(1)
                        }
                    };
                    ClientStorage::store_processed_data(
                        store_data,
                        processing_data_lock.store_dir.as_path(),
                    )
                    .unwrap_or_else(|e| {
                        error!("failed to store processed sphinx message; err = {:?}", e);
                    });
                }
                Err(e) => {
                    warn!("failed to read from socket; err = {:?}", e);
                    return;
                }
            };

            // Write the some data back
            if let Err(e) = socket.write_all(b"foomp").await {
                warn!("failed to write reply to socket; err = {:?}", e);
                return;
            }
        }
    }

    async fn send_response(mut socket: tokio::net::TcpStream, data: &[u8]) {
        if let Err(e) = socket.write_all(data).await {
            warn!("failed to write reply to socket; err = {:?}", e)
        }
        if let Err(e) = socket.shutdown(Shutdown::Write) {
            warn!("failed to close write part of the socket; err = {:?}", e)
        }
    }

    // TODO: FIGURE OUT HOW TO SET READ_DEADLINES IN TOKIO
    async fn process_client_socket_connection(
        mut socket: tokio::net::TcpStream,
        processing_data: Arc<ClientProcessingData>,
    ) {
        let mut buf = [0; 1024];

        // TODO: restore the for loop once we go back to persistent tcp socket connection
        let response = match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                trace!("Remote connection closed.");
                Err(())
            }
            Ok(n) => {
                match ClientRequestProcessor::process_client_request(
                    buf[..n].as_ref(),
                    processing_data,
                )
                .await
                {
                    Err(e) => {
                        warn!("failed to process client request; err = {:?}", e);
                        Err(())
                    }
                    Ok(res) => Ok(res),
                }
            }
            Err(e) => {
                warn!("failed to read from socket; err = {:?}", e);
                Err(())
            }
        };

        if let Err(e) = socket.shutdown(Shutdown::Read) {
            warn!("failed to close read part of the socket; err = {:?}", e)
        }

        match response {
            Ok(res) => {
                ServiceProvider::send_response(socket, &res).await;
            }
            _ => {
                ServiceProvider::send_response(socket, b"bad foomp").await;
            }
        }
    }

    async fn start_mixnet_listening(
        address: SocketAddr,
        secret_key: encryption::PrivateKey,
        store_dir: PathBuf,
        new_messages_filename_length: u16,
    ) -> Result<(), ProviderError> {
        let mut listener = tokio::net::TcpListener::bind(address).await?;
        let processing_data =
            MixProcessingData::new(secret_key, store_dir, new_messages_filename_length)
                .add_arc_rwlock();

        loop {
            let (socket, _) = listener.accept().await?;
            // do note that the underlying data is NOT copied here; arc is incremented and lock is shared
            // (if I understand it all correctly)
            let thread_processing_data = processing_data.clone();
            tokio::spawn(async move {
                ServiceProvider::process_mixnet_socket_connection(socket, thread_processing_data)
                    .await
            });
        }
    }

    async fn start_client_listening(
        address: SocketAddr,
        store_dir: PathBuf,
        client_ledger: Arc<FMutex<ClientLedger>>,
        secret_key: encryption::PrivateKey,
        message_retrieval_limit: u16,
    ) -> Result<(), ProviderError> {
        let mut listener = tokio::net::TcpListener::bind(address).await?;
        let processing_data = ClientProcessingData::new(
            store_dir,
            client_ledger,
            secret_key,
            message_retrieval_limit,
        )
        .add_arc();

        loop {
            let (socket, _) = listener.accept().await?;
            // do note that the underlying data is NOT copied here; arc is incremented and lock is shared
            // (if I understand it all correctly)
            let thread_processing_data = processing_data.clone();
            tokio::spawn(async move {
                ServiceProvider::process_client_socket_connection(socket, thread_processing_data)
                    .await
            });
        }
    }

    pub fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to Provider struct itself?
        // TODO: figure out the difference between Runtime and Handle
        let mut rt = Runtime::new()?;
        //        let mut h = rt.handle();

        let initial_client_ledger = self.registered_clients_ledger;
        let thread_shareable_ledger = initial_client_ledger.add_arc_futures_mutex();

        let notifier_config = presence::NotifierConfig::new(
            self.config.get_presence_directory_server(),
            self.config.get_mix_announce_address(),
            self.config.get_clients_announce_address(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_presence_sending_delay(),
        );

        let presence_future = rt.spawn({
            let presence_notifier =
                presence::Notifier::new(notifier_config, thread_shareable_ledger.clone());
            presence_notifier.run()
        });

        let mix_future = rt.spawn(ServiceProvider::start_mixnet_listening(
            self.config.get_mix_listening_address(),
            self.sphinx_keypair.private_key().clone(), // CLONE IS DONE TEMPORARILY UNTIL PROVIDER IS REFACTORED THE MIXNODE STYLE
            self.config.get_clients_inboxes_dir(),
            self.config.get_stored_messages_filename_length(),
        ));
        let client_future = rt.spawn(ServiceProvider::start_client_listening(
            self.config.get_clients_listening_address(),
            self.config.get_clients_inboxes_dir(),
            thread_shareable_ledger,
            self.sphinx_keypair.private_key().clone(), // CLONE IS DONE TEMPORARILY UNTIL PROVIDER IS REFACTORED THE MIXNODE STYLE
            self.config.get_message_retrieval_limit(),
        ));
        // Spawn the root task
        rt.block_on(async {
            let future_results =
                futures::future::join3(mix_future, client_future, presence_future).await;
            assert!(future_results.0.is_ok() && future_results.1.is_ok());
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on listeners
        error!("The server went kaput...");
        Ok(())
    }
}
