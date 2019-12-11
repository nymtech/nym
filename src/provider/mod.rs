use std::fs::File;
use std::io::Write;
use std::net::{SocketAddr, Shutdown};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use sfw_provider_requests::*;
use curve25519_dalek::scalar::Scalar;
use rand::Rng;
use sphinx::{ProcessedPacket, SphinxPacket};
use sphinx::route::{DestinationAddressBytes, SURBIdentifier};
use tokio::prelude::*;
use tokio::runtime::Runtime;
use std::time::Duration;
use futures::{Future, TryFutureExt};

// TODO: if we ever create config file, this should go there
const STORED_MESSAGE_FILENAME_LENGTH: usize = 16;


// TODO: this will probably need to be moved elsewhere I imagine
// DUPLICATE WITH MIXNODE CODE!!!
#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedForwardHopError,
    InvalidPayload,
    NonMatchingRecipient,
    FileIOFailure,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxRecoveryError
    }
}

impl From<std::io::Error> for MixProcessingError {
    fn from(_: std::io::Error) -> Self {
        use MixProcessingError::*;

        FileIOFailure
    }
}


// ProcessingData defines all data required to correctly unwrap sphinx packets
// Do note that we're copying this struct around and hence the secret_key.
// It might, or might not be, what we want
#[derive(Debug, Clone)]
struct MixProcessingData {
    secret_key: Scalar,
    store_dir: PathBuf,
}

impl MixProcessingData {
    fn new(secret_key: Scalar, store_dir: PathBuf) -> Self {
        MixProcessingData {
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


struct MixPacketProcessor(());

impl MixPacketProcessor {
    fn process_sphinx_data_packet(packet_data: &[u8], processing_data: &RwLock<MixProcessingData>) -> Result<StoreData, MixProcessingError> {
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

    fn generate_random_file_name() -> String {
        rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(STORED_MESSAGE_FILENAME_LENGTH).collect::<String>()
    }

    fn store_processed_data(store_data: StoreData, store_dir: &Path) -> Result<(), MixProcessingError> {
        let client_dir_name = hex::encode(store_data.client_address);
        let full_store_dir = store_dir.join(client_dir_name);
        let full_store_path = full_store_dir.join(MixPacketProcessor::generate_random_file_name());
        println!("going to store: {:?} in file: {:?}", store_data.message, full_store_path);

        // TODO: what to do with surbIDs??

        // we can use normal io here, no need for tokio as it's all happening in one thread per connection
        std::fs::create_dir_all(full_store_dir)?;
        let mut file = File::create(full_store_path)?;
        file.write_all(store_data.message.as_ref())?;


        Ok(())
    }
}

#[derive(Debug)]
enum ClientProcessingError {
    ClientDoesntExistError
}

struct ClientRequestProcessor(());

impl ClientRequestProcessor {
    fn process_client_request(data: &[u8]) -> Result<Vec<u8>, ClientProcessingError> {
        println!("received the following data: {:?}", data);
        let semi_parsed = ProviderRequests::from_bytes(&data);
        println!("received the following data: {:?}", semi_parsed);


        Ok(vec![42])
    }
}


pub struct ServiceProvider {
    mix_network_address: SocketAddr,
    client_network_address: SocketAddr,
    secret_key: Scalar,
    store_dir: PathBuf,
}

impl ServiceProvider {
    pub fn new(mix_network_address: SocketAddr, client_network_address: SocketAddr, secret_key: Scalar, store_dir: PathBuf) -> Self {
        ServiceProvider {
            mix_network_address,
            client_network_address,
            secret_key,
            store_dir,
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
                    MixPacketProcessor::store_processed_data(store_data, processing_data.read().unwrap().store_dir.as_path()).unwrap_or_else(|e| {
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

    async fn process_client_socket_connection(mut socket: tokio::net::TcpStream) {
        let mut buf = Vec::new();

        // TODO: restore the for loop once we go back to persistent tcp socket connection
        let response = match socket.read_to_end(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                println!("Remote connection closed.");
                Err(())
            }
            Ok(_) => {
                match ClientRequestProcessor::process_client_request(buf.as_ref()) {
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
                ServiceProvider::send_response(socket, b"good foomp").await;
            },
            _ => {
                println!("we failed...");
                ServiceProvider::send_response(socket, b"bad foomp").await;
            },
        }
    }

    async fn start_mixnet_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener = tokio::net::TcpListener::bind(self.mix_network_address).await?;
        let processing_data = MixProcessingData::new(self.secret_key, self.store_dir.clone()).add_arc_rwlock();

        loop {
            let (socket, _) = listener.accept().await?;
            let thread_processing_data = processing_data.clone();
            tokio::spawn(async move {
                ServiceProvider::process_mixnet_socket_connection(socket, thread_processing_data).await
            });
        }
    }

    async fn start_client_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener = tokio::net::TcpListener::bind(self.client_network_address).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            println!("new connection!");
            tokio::spawn(async move {
                ServiceProvider::process_client_socket_connection(socket).await
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




