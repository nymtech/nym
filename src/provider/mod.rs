use std::net::{Shutdown, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use crate::provider::client_handling::{ClientProcessingData, ClientRequestProcessor};
use crate::provider::mix_handling::{MixPacketProcessor, MixProcessingData};
use crate::provider::storage::ClientStorage;
use futures::io::Error;
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use std::collections::HashMap;
use futures::lock::Mutex as FMutex;

mod client_handling;
mod mix_handling;
pub mod presence;
mod storage;

// TODO: if we ever create config file, this should go there
const STORED_MESSAGE_FILENAME_LENGTH: usize = 16;
const MESSAGE_RETRIEVAL_LIMIT: usize = 2;

pub struct Config {
    pub client_socket_address: SocketAddr,
    pub directory_server: String,
    pub mix_socket_address: SocketAddr,
    pub public_key: MontgomeryPoint,
    pub secret_key: Scalar,
    pub store_dir: PathBuf,
}

impl Config {
    pub fn public_key_string(&self) -> String {
        let key_bytes = self.public_key.to_bytes().to_vec();
        base64::encode_config(&key_bytes, base64::URL_SAFE)
    }
}

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

    fn has_token(&self, auth_token: AuthToken) -> bool {
        return self.0.contains_key(&auth_token)
    }

    fn insert_token(&mut self, auth_token: AuthToken, client_address: DestinationAddressBytes) -> Option<DestinationAddressBytes>{
        self.0.insert(auth_token, client_address)
    }

    #[allow(dead_code)]
    fn load(_file: PathBuf) -> Self {
        unimplemented!()
    }
}

pub struct ServiceProvider {
    mix_network_address: SocketAddr,
    client_network_address: SocketAddr,
    secret_key: Scalar,
    store_dir: PathBuf,
    registered_clients_ledger: ClientLedger,
}

impl ServiceProvider {
    pub fn new(config: &Config) -> Self {
        ServiceProvider {
            mix_network_address: config.mix_socket_address,
            client_network_address: config.client_socket_address,
            secret_key: config.secret_key,
            store_dir: PathBuf::from(config.store_dir.clone()),
            // TODO: load initial ledger from file
            registered_clients_ledger: ClientLedger::new(),
        }
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
                    println!("Remote connection closed.");
                    return;
                }
                Ok(_) => {
                    let store_data = match MixPacketProcessor::process_sphinx_data_packet(
                        buf.as_ref(),
                        processing_data.as_ref(),
                    ) {
                        Ok(sd) => sd,
                        Err(e) => {
                            eprintln!("failed to process sphinx packet; err = {:?}", e);
                            return;
                        }
                    };
                    ClientStorage::store_processed_data(
                        store_data,
                        processing_data.read().unwrap().store_dir.as_path(),
                    )
                        .unwrap_or_else(|e| {
                            eprintln!("failed to store processed sphinx message; err = {:?}", e);
                            return;
                        });
                }
                Err(e) => {
                    eprintln!("failed to read from socket; err = {:?}", e);
                    return;
                }
            };

            // Write the some data back
            if let Err(e) = socket.write_all(b"foomp").await {
                eprintln!("failed to write reply to socket; err = {:?}", e);
                return;
            }
        }
    }

    async fn send_response(mut socket: tokio::net::TcpStream, data: &[u8]) {
        if let Err(e) = socket.write_all(data).await {
            eprintln!("failed to write reply to socket; err = {:?}", e)
        }
        if let Err(e) = socket.shutdown(Shutdown::Write) {
            eprintln!("failed to close write part of the socket; err = {:?}", e)
        }
    }

    // TODO: FIGURE OUT HOW TO SET READ_DEADLINES IN TOKIO
    async fn process_client_socket_connection(
        mut socket: tokio::net::TcpStream,
        processing_data: Arc<FMutex<ClientProcessingData>>,
    ) {
        let mut buf = [0; 1024];

        // TODO: restore the for loop once we go back to persistent tcp socket connection
        let response = match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                println!("Remote connection closed.");
                Err(())
            }
            Ok(n) => {
                match ClientRequestProcessor::process_client_request(
                    buf[..n].as_ref(),
                    processing_data,
                ).await {
                    Err(e) => {
                        eprintln!("failed to process client request; err = {:?}", e);
                        Err(())
                    }
                    Ok(res) => Ok(res),
                }
            }
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                Err(())
            }
        };

        if let Err(e) = socket.shutdown(Shutdown::Read) {
            eprintln!("failed to close read part of the socket; err = {:?}", e)
        }

        match response {
            Ok(res) => {
                println!("should send this response! {:?}", res);
                ServiceProvider::send_response(socket, &res).await;
            }
            _ => {
                println!("we failed...");
                ServiceProvider::send_response(socket, b"bad foomp").await;
            }
        }
    }

    async fn start_mixnet_listening(
        address: SocketAddr,
        secret_key: Scalar,
        store_dir: PathBuf,
    ) -> Result<(), ProviderError> {
        let mut listener = tokio::net::TcpListener::bind(address).await?;
        let processing_data = MixProcessingData::new(secret_key, store_dir).add_arc_rwlock();

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
        client_ledger: ClientLedger,
        secret_key: Scalar,
    ) -> Result<(), ProviderError> {
        let mut listener = tokio::net::TcpListener::bind(address).await?;
        let processing_data =
            ClientProcessingData::new(store_dir, client_ledger, secret_key).add_arc_futures_mutex();

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

    // Note: this now consumes the provider
    pub fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to Provider struct itself?
        // TODO: figure out the difference between Runtime and Handle
        let mut rt = Runtime::new()?;
        //        let mut h = rt.handle();

        let mix_future = rt.spawn(ServiceProvider::start_mixnet_listening(
            self.mix_network_address,
            self.secret_key,
            self.store_dir.clone(),
        ));
        let client_future = rt.spawn(ServiceProvider::start_client_listening(
            self.client_network_address,
            self.store_dir.clone(),
            self.registered_clients_ledger, // we're just cloning the initial ledger state
            self.secret_key,
        ));
        // Spawn the root task
        rt.block_on(async {
            let future_results = futures::future::join(mix_future, client_future).await;
            assert!(future_results.0.is_ok() && future_results.1.is_ok());
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on listeners
        eprintln!("The server went kaput...");
        Ok(())
    }
}
