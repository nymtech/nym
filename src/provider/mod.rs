use std::net::{Shutdown, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use curve25519_dalek::scalar::Scalar;
use tokio::prelude::*;
use tokio::runtime::Runtime;

use crate::provider::client_handling::{ClientProcessingData, ClientRequestProcessor};
use crate::provider::mix_handling::{MixPacketProcessor, MixProcessingData};
use crate::provider::storage::ClientStorage;
use std::collections::HashMap;
use sphinx::route::DestinationAddressBytes;

mod client_handling;
mod mix_handling;
mod storage;


// TODO: if we ever create config file, this should go there
const STORED_MESSAGE_FILENAME_LENGTH: usize = 16;
const MESSAGE_RETRIEVAL_LIMIT: usize = 2;

pub struct ServiceProvider {
    mix_network_address: SocketAddr,
    client_network_address: SocketAddr,
    secret_key: Scalar,
    store_dir: PathBuf,
    registered_clients_ledger: HashMap<DestinationAddressBytes, Vec<u8>>,
}

impl ServiceProvider {
    pub fn new(mix_network_address: SocketAddr, client_network_address: SocketAddr, secret_key: Scalar, store_dir: PathBuf) -> Self {

        let mut registered_clients_ledger = HashMap::new();
        ServiceProvider {
            mix_network_address,
            client_network_address,
            secret_key,
            store_dir,
            registered_clients_ledger,
        }
    }


    async fn process_mixnet_socket_connection(mut socket: tokio::net::TcpStream, processing_data: Arc<RwLock<MixProcessingData>>) {
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
                    let store_data = match MixPacketProcessor::process_sphinx_data_packet(buf.as_ref(), processing_data.as_ref()) {
                        Ok(sd) => sd,
                        Err(e) => {
                            eprintln!("failed to process sphinx packet; err = {:?}", e);
                            return;
                        }
                    };
                    ClientStorage::store_processed_data(store_data, processing_data.read().unwrap().store_dir.as_path()).unwrap_or_else(|e| {
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
    async fn process_client_socket_connection(mut socket: tokio::net::TcpStream, processing_data: Arc<RwLock<ClientProcessingData>>, secret_key: Scalar) {
        let mut buf = [0; 1024];

        // TODO: restore the for loop once we go back to persistent tcp socket connection
        let response = match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                println!("Remote connection closed.");
                Err(())
            }
            Ok(n) => {
                match ClientRequestProcessor::process_client_request(buf[..n].as_ref(), processing_data.as_ref(), secret_key) {
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

    async fn start_mixnet_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener = tokio::net::TcpListener::bind(self.mix_network_address).await?;
        let processing_data = MixProcessingData::new(self.secret_key, self.store_dir.clone()).add_arc_rwlock();

        loop {
            let (socket, _) = listener.accept().await?;
            // do note that the underlying data is NOT copied here; arc is incremented and lock is shared
            // (if I understand it all correctly)
            let thread_processing_data = processing_data.clone();
            tokio::spawn(async move {
                ServiceProvider::process_mixnet_socket_connection(socket, thread_processing_data).await
            });
        }
    }

    async fn start_client_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener = tokio::net::TcpListener::bind(self.client_network_address).await?;
        let processing_data = ClientProcessingData::new(self.store_dir.clone(), self.registered_clients_ledger.clone()).add_arc_rwlock();
        let key = self.secret_key.clone();
        loop {
            let (socket, _) = listener.accept().await?;
            // do note that the underlying data is NOT copied here; arc is incremented and lock is shared
            // (if I understand it all correctly)
            let thread_processing_data = processing_data.clone();
            tokio::spawn(async move {
                ServiceProvider::process_client_socket_connection(socket, thread_processing_data, key).await
            });
        }
    }

    async fn start_listeners(&self) -> (Result<(), Box<dyn std::error::Error>>, Result<(), Box<dyn std::error::Error>>) {
        futures::future::join(self.start_mixnet_listening(), self.start_client_listening()).await
    }


    pub fn start_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to Provider struct itself?
        // TODO: figure out the difference between Runtime and Handle
        let mut rt = Runtime::new()?;
//        let mut h = rt.handle();

        // Spawn the root task
        rt.block_on(async {
            let future_results = self.start_listeners().await;
            assert!(future_results.0.is_ok() && future_results.1.is_ok())
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on listeners
        eprintln!("The server went kaput...");
        Ok(())
    }
}




