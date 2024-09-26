use crate::mixnet::{
    AnonymousSenderTag, MixnetClient, MixnetClientBuilder, MixnetClientSender, MixnetMessageSender,
    NymNetworkDetails, StoragePaths,
};
use anyhow::Result;
use dashmap::DashSet;
use nym_network_defaults::setup_env;
use nym_sphinx::addressing::Recipient;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::watch::Receiver;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};
#[allow(clippy::duplicate_mod)]
#[path = "utils.rs"]
mod utils;
use utils::{MessageBuffer, Payload, ProxiedMessage};
use uuid::Uuid;

pub struct NymProxyServer {
    upstream_address: String,
    session_map: DashSet<Uuid>,
    mixnet_client: MixnetClient,
    mixnet_client_sender: Arc<RwLock<MixnetClientSender>>,
    tx: tokio::sync::watch::Sender<Option<(ProxiedMessage, AnonymousSenderTag)>>,
    rx: tokio::sync::watch::Receiver<Option<(ProxiedMessage, AnonymousSenderTag)>>,
}

impl NymProxyServer {
    pub async fn new(
        upstream_address: &str,
        config_dir: &str,
        env: Option<String>,
    ) -> Result<Self> {
        info!(":: creating client...");

        // We're wanting to build a client with a constant address, vs the ephemeral in-memory data storage of the NymProxyClient clients.
        // Following a builder pattern, having to manually connect to the mixnet below.
        let config_dir = PathBuf::from(config_dir);
        debug!("loading env file: {:?}", env);
        setup_env(env); // Defaults to mainnet if empty
        let net = NymNetworkDetails::new_from_env();
        let storage_paths = StoragePaths::new_from_dir(&config_dir)?;
        let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
            .await?
            .network_details(net)
            .build()?;

        let client = client.connect_to_mixnet().await?;

        // Since we're splitting the client in the main thread, we have to wrap the sender side of the client in an Arc<RwLock>>.
        let sender = Arc::new(RwLock::new(client.split_sender()));

        // Used for passing the incoming Mixnet message => session_handler().
        let (tx, rx) =
            tokio::sync::watch::channel::<Option<(ProxiedMessage, AnonymousSenderTag)>>(None);

        info!(":: client created: {}", client.nym_address());

        Ok(NymProxyServer {
            upstream_address: upstream_address.to_string(),
            session_map: DashSet::new(),
            mixnet_client: client,
            mixnet_client_sender: sender,
            tx,
            rx,
        })
    }

    pub fn nym_address(&self) -> &Recipient {
        self.mixnet_client.nym_address()
    }

    pub fn mixnet_client_mut(&mut self) -> &mut MixnetClient {
        &mut self.mixnet_client
    }

    pub fn session_map(&self) -> &DashSet<Uuid> {
        &self.session_map
    }

    pub fn mixnet_client_sender(&self) -> Arc<RwLock<MixnetClientSender>> {
        Arc::clone(&self.mixnet_client_sender)
    }

    pub fn tx(&self) -> tokio::sync::watch::Sender<Option<(ProxiedMessage, AnonymousSenderTag)>> {
        self.tx.clone()
    }

    pub fn rx(&self) -> tokio::sync::watch::Receiver<Option<(ProxiedMessage, AnonymousSenderTag)>> {
        self.rx.clone()
    }

    // The main body of our logic, triggered on each received new sessionID. To deal with assumptions about
    // streaming we have to implement an abstract session for each set of outgoing messages atop each connection, with message
    // IDs to deal with the fact that the mixnet does not enforce message ordering.
    //
    // There is an initial thread which does a bunch of setup logic:
    //      - Create a TcpStream connecting to our upstream server process.
    //      - Split incoming TcpStream into OwnedReadHalf and OwnedWriteHalf for concurrent read/write.
    //      - Create an Arc to store our session SURB - used for anonymous replies.
    //
    // Then we spawn 2 tasks:
    // - 'Incoming' thread => deals with parsing and storing the SURB (used in Mixnet replies), deserialising and passing the incoming data from the Mixnet to the upstream server.
    // - 'Outgoing' thread => frames bytes coming from TcpStream (the server) and deals with ordering + sending reply anonymously => Mixnet.
    async fn session_handler(
        upstream_address: String,
        session_id: Uuid,
        mut rx: Receiver<Option<(ProxiedMessage, AnonymousSenderTag)>>,
        sender: Arc<RwLock<MixnetClientSender>>,
    ) -> Result<()> {
        let global_surb = Arc::new(RwLock::new(None));
        let stream = TcpStream::connect(upstream_address).await?;

        // Split our tcpstream into OwnedRead and OwnedWrite halves for concurrent read/writing.
        let (read, mut write) = stream.into_split();
        // Used for anonymous replies per session. Initially parsed from the incoming message.
        let send_side_surb = Arc::clone(&global_surb);

        tokio::spawn(async move {
            let mut message_id = 0;
            // Since we're just trying to pipe whatever bytes our client/server are normally sending to each other,
            // the bytescodec is fine to use here; we're trying to avoid modifying this stream e.g. in the process of Sphinx packet
            // creation and adding padding to the payload whilst also sidestepping the need to manually manage an intermediate buffer of the
            // incoming bytes from the tcp stream and writing them to our server with our Nym client.
            let codec = tokio_util::codec::BytesCodec::new();
            let mut framed_read = tokio_util::codec::FramedRead::new(read, codec);

            // While able to read from OwnedReadHalf of TcpStream:
            // - Keep track of outgoing messageIDs.
            // - Read and store incoming SURB.
            // - Send serialised reply => Mixnet via SURB.
            // - If tick() returns true, close session.
            while let Some(Ok(bytes)) = framed_read.next().await {
                info!("server received {} bytes", bytes.len());
                let reply =
                    ProxiedMessage::new(Payload::Data(bytes.to_vec()), session_id, message_id);
                message_id += 1;
                let surb = send_side_surb.read().await;
                if let Some(surb) = *surb {
                    sender
                        .write()
                        .await
                        .send_reply(surb, bincode::serialize(&reply)?)
                        .await?
                }
                info!(
                    "Sent reply with id {} for session {}",
                    message_id, session_id
                );
            }
            Ok::<(), anyhow::Error>(())
        });

        let messages_accounter = Arc::new(DashSet::new());
        messages_accounter.insert(1);

        let mut msg_buffer = MessageBuffer::new();
        loop {
            tokio::select! {
                    _ = rx.changed() => {
                        let value = rx.borrow_and_update().clone();
                        if let Some((message, surb)) = value {
                            if message.session_id() != session_id {
                                continue;
                            }

                            msg_buffer.push(message);

                            let local_surb = Arc::clone(&global_surb);
                            {
                                *local_surb.write().await = Some(surb);
                            }

                            let should_close = msg_buffer.tick(&mut write).await?;
                            if should_close {
                                info!(":: Closing write end of session: {}", session_id);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        msg_buffer.tick(&mut write).await?;
                    }
            }
        }
        // This times out after 60 seconds by default.
        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn run_with_shutdown(&mut self) -> Result<()> {
        // On our Mixnet client getting a new message:
        // - Check if the attached sessionID exists.
        // - If !sessionID, spawn a new session_handler() task.
        // - Send the message down tx => rx in our handler.
        while let Some(new_message) = &self.mixnet_client_mut().next().await {
            let message: ProxiedMessage = bincode::deserialize(&new_message.message)?;
            let session_id = message.session_id();
            // If we've already got message from an existing session, continue, else add it to the session mapping and spawn a new handler().
            if self.session_map().contains(&message.session_id()) {
                debug!("Got message for an existing session");
            } else {
                self.session_map().insert(message.session_id());
                debug!("Got message for a new session");
                tokio::spawn(Self::session_handler(
                    self.upstream_address.clone(),
                    session_id,
                    self.rx(),
                    self.mixnet_client_sender(),
                ));
                info!("Spawned a new session handler: {}", message.session_id());
            }

            debug!("Sending message for session {}", message.session_id());

            if let Some(sender_tag) = new_message.sender_tag {
                self.tx.send(Some((message, sender_tag)))?
            } else {
                error!("No sender tag found, we can't send a reply without it!")
            }
        }

        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}
