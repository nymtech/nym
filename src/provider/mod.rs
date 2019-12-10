use std::borrow::Borrow;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

use curve25519_dalek::scalar::Scalar;
use futures::StreamExt;
use sphinx::{ProcessedPacket, SphinxPacket};
use sphinx::route::{DestinationAddressBytes, SURBIdentifier};
use tokio::prelude::*;
use tokio::runtime::Runtime;

// TODO: this will probably need to be moved elsewhere I imagine
// DUPLICATE WITH MIXNODE CODE!!!
#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedForwardHopError,
    InvalidPayload,
    NonMatchingRecipient,
    StoreFailure,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxRecoveryError
    }
}

// ProcessingData defines all data required to correctly unwrap sphinx packets
// Do note that we're copying this struct around and hence the secret_key.
// It might, or might not be, what we want
#[derive(Debug, Clone)]
struct ProcessingData {
    secret_key: Scalar,
    store_dir: PathBuf,
}

impl ProcessingData {
    fn new(secret_key: Scalar, store_dir: PathBuf) -> Self {
        ProcessingData {
            secret_key,
            store_dir,
        }
    }

    fn add_arc_rwlock(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }
}

struct StoreData {
    client_address: DestinationAddressBytes,
    client_surb_id: SURBIdentifier,
    message: Vec<u8>,
}


struct PacketProcessor(());

impl PacketProcessor {
    fn process_sphinx_data_packet(packet_data: &[u8], processing_data: &RwLock<ProcessingData>) -> Result<StoreData, MixProcessingError> {
        let packet = SphinxPacket::from_bytes(packet_data.to_vec())?;
        let read_processing_data = processing_data.read().unwrap();
        let (client_address, client_surb_id, payload) = match packet.process(read_processing_data.secret_key) {
            ProcessedPacket::ProcessedPacketFinalHop(client_address, surb_id, payload) => (client_address, surb_id, payload),
            _ => return Err(MixProcessingError::ReceivedForwardHopError),
        };

        let (payload_destination, message) = payload.try_recover_destination_and_plaintext().ok_or_else(|| MixProcessingError::InvalidPayload)?;
        if client_address != payload_destination {
            return Err(MixProcessingError::NonMatchingRecipient);
        }

        Ok(StoreData {
            client_address,
            client_surb_id,
            message,
        })
    }

    fn store_processed_data(store_data: StoreData, store_dir: &Path) -> Result<(), MixProcessingError> {
        println!("going to store: {:?} in base dir: {:?}", store_data.message, store_dir);
        Ok(())
    }
}


pub struct ServiceProvider {
    network_address: SocketAddr,
    secret_key: Scalar,
    store_dir: PathBuf,
}

impl ServiceProvider {
    pub fn new(network_address: SocketAddr, secret_key: Scalar, store_dir: PathBuf) -> Self {
        ServiceProvider {
            network_address,
            secret_key,
            store_dir,
        }
    }


    async fn process_socket_connection(mut socket: tokio::net::TcpStream, processing_data: Arc<RwLock<ProcessingData>>) {
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
                    let store_data = match PacketProcessor::process_sphinx_data_packet(buf.as_ref(), processing_data.as_ref()) {
                        Ok(sd) => sd,
                        Err(e) => {
                            eprintln!("failed to process sphinx packet; err = {:?}", e);
                            return;
                        }
                    };
                    PacketProcessor::store_processed_data(store_data, processing_data.read().unwrap().store_dir.as_path()).unwrap_or_else(|e| {
                        eprintln!("failed to store processed sphinx message; err = {:?}", e)
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


    pub fn start_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to Provider struct itself?
        // TODO: figure out the difference between Runtime and Handle
        let mut rt = Runtime::new()?;
//        let mut h = rt.handle();

        // Spawn the root task
        rt.block_on(async {
            let mut listener = tokio::net::TcpListener::bind(self.network_address).await?;
            let processing_data = ProcessingData::new(self.secret_key, self.store_dir.clone()).add_arc_rwlock();

            loop {
                let (socket, _) = listener.accept().await?;
                let thread_processing_data = processing_data.clone();
                tokio::spawn(async move {
                    ServiceProvider::process_socket_connection(socket, thread_processing_data).await
                });
            }

//            async move {
//                let mut incoming = listener.incoming();
//
//                while let Some(conn) = incoming.next().await {
//                    match conn {
//                        Err(e) => eprintln!("accept failed with error: {:?}", e),
//                        Ok(socket) => {
//                            let foomp2 = processing_data_foomp.clone();
//                            tokio::spawn(async move {
//                                ServiceProvider::process_socket_connection(socket, foomp2).await
//
////                                ServiceProvider::process_socket_fixture(socket).await
//                            });
//                        }
//                    }
//                }
//
//            }.await;
//
//            println!("Server went kaput");
//            Ok(())


////            let processing_data = Arc::new(RwLock::new(ProcessingData::new(self.secret_key, self.store_dir.clone())));
//            let processing_data = Arc::new(RwLock::new((ProcessingData::new(self.secret_key))));
//
//            loop {
//                let (socket, _) = listener.accept().await?;
//
////                tokio::
//                tokio::spawn(async move {
////                    processing_data.read();
////                    let foo = processing_data.clone();
////                    let foo = ProcessingData::new(self.secret_key.clone());
//                    // Process each socket concurrently.
////                    ServiceProvider::process_socket_connection(socket, processing_data).await
//                });
//            }
        })
    }
}




