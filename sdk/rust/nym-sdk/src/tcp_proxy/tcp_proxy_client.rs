use crate::mixnet::{
    IncludedSurbs, MixnetClient, MixnetClientBuilder, MixnetMessageSender, NymNetworkDetails,
};
use std::sync::Arc;
#[path = "utils.rs"]
mod utils;
use anyhow::Result;
use dashmap::DashSet;
use nym_network_defaults::setup_env;
use nym_network_defaults::var_names::NYM_API;
use nym_sphinx::addressing::Recipient;
use nym_topology::{HardcodedTopologyProvider, NymTopology};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{debug, info, instrument, warn};
use utils::{MessageBuffer, Payload, ProxiedMessage};

const DEFAULT_CLOSE_TIMEOUT: u64 = 60;
const DEFAULT_LISTEN_HOST: &str = "127.0.0.1";
const DEFAULT_LISTEN_PORT: &str = "8080";

pub struct NymProxyClient {
    server_address: Recipient,
    listen_address: String,
    listen_port: String,
    close_timeout: u64,
}

impl NymProxyClient {
    pub async fn new(
        server_address: Recipient,
        listen_address: &str,
        listen_port: &str,
        close_timeout: u64,
        env: Option<String>,
    ) -> Result<Self> {
        debug!("loading env file: {:?}", env);
        setup_env(env);
        Ok(NymProxyClient {
            server_address,
            listen_address: listen_address.to_string(),
            listen_port: listen_port.to_string(),
            close_timeout,
        })
    }

    // server_address is the Nym address of the NymProxyServer to communicate with.
    pub async fn new_with_defaults(server_address: Recipient, env: Option<String>) -> Result<Self> {
        debug!("loading env file: {:?}", env);
        setup_env(env);
        Ok(NymProxyClient {
            server_address,
            listen_address: DEFAULT_LISTEN_HOST.to_string(),
            listen_port: DEFAULT_LISTEN_PORT.to_string(),
            close_timeout: DEFAULT_CLOSE_TIMEOUT,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Connecting to mixnet server at {}", self.server_address);

        let listener =
            TcpListener::bind(format!("{}:{}", self.listen_address, self.listen_port)).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(NymProxyClient::handle_incoming(
                stream,
                self.server_address.clone(),
                self.close_timeout,
            ));
        }
    }

    // The main body of our logic, triggered on each accepted incoming tcp connection. To deal with assumptions about
    // streaming we have to implement an abstract session for each set of outgoing messages atop each connection, with message
    // IDs to deal with the fact that the mixnet does not enforce message ordering.
    //
    // There is an initial thread which does a bunch of setup logic
    // - Create a random session ID
    // - Create a Nym Client (and split into read/write clients for concurrent read/write)
    // - Split incoming TcpStream into OwnedReadHalf and OwnedWriteHalf for concurrent read/write
    //
    // Then we spawn 2 tasks:
    // - 'Outgoing' thread => frames incoming bytes from OwnedReadHalf and pipe through the mixnet & trigger session close.
    // - 'Incoming' thread => orders incoming messages from the Mixnet via placing them in a MessageBuffer and using tick(), as well as manage session closing.
    #[instrument]
    async fn handle_incoming(
        stream: TcpStream,
        server_address: Recipient,
        close_timeout: u64,
    ) -> Result<()> {
        // ID for creation of session abstraction; new session ID per new connection accepted by our tcp listener above.
        let session_id = uuid::Uuid::new_v4();

        // Used to communicate end of session between 'Outgoing' and 'Incoming' tasks
        let (tx, mut rx) = oneshot::channel();

        // Client creation can fail for multiple reasons like bad network connection: this loop just allows us to
        // retry in a loop until we can successfully connect without having to restart the entire function
        info!(":: Starting session: {}", session_id);
        info!(":: creating client...");
        let mut client = loop {
            let net = NymNetworkDetails::new_from_env();
            // TODO change to builder but ephemeral
            // match MixnetClient::connect_new().await {
            match MixnetClientBuilder::new_ephemeral()
                .network_details(net)
                .build()?
                .connect_to_mixnet()
                .await
            {
                Ok(client) => break client,
                Err(err) => {
                    warn!(":: Error creating client: {:?}, will retry in 100ms", err);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        };
        let client_addr = &client.nym_address();
        info!(":: client created: {}", &client_addr);

        // Split our tcpstream into OwnedRead and OwnedWrite halves for concurrent read/writing
        let (read, mut write) = stream.into_split();
        // Since we're just trying to pipe whatever bytes our client/server are normally sending to each other,
        // the bytescodec is fine to use here; we're trying to avoid modifying this stream e.g. in the process of Sphinx packet
        // creation and adding padding to the payload whilst also sidestepping the need to manually manage an intermediate buffer of the
        // incoming bytes from the tcp stream and writing them to our server with our Nym client.
        let codec = BytesCodec::new();
        let mut framed_read = FramedRead::new(read, codec);
        // Much like the tcpstream, split our Nym client into a sender and receiver for concurrent read/write
        let sender = client.split_sender();
        // The server / service provider address our client is sending messages to will remain static
        let server_addr = server_address.clone();
        // Store outgoing messages in instance of Dashset abstraction
        let messages_account = Arc::new(DashSet::new());
        // Wrap in an Arc for memsafe concurrent access
        let sent_messages_account = Arc::clone(&messages_account);

        // 'Outgoing' thread
        tokio::spawn(async move {
            let mut message_id = 0;
            // While able to read from OwnedReadHalf of TcpStream:
            // - increment our messageID - we need to ensure message ordering on both client and server.
            // - create instance of ProxiedMessage abstraction with framed bytes: this is really just the message data payload in the form of those bytes
            //   & session and messageIDs.
            // - Serialise + send message through the mixnet to the Service Provider.
            // - Repeat these steps, but sending a message with a payload containing a Close signal for this session; since we have message ordering implemented
            //   we can fire off the session close signal without having to wait on making sure the server has received the rest of the messages.
            // - Trigger our session timeout alert in the 'Incoming' thread select! loop via tx end of our oneshot channel.
            while let Some(Ok(bytes)) = framed_read.next().await {
                message_id += 1;
                sent_messages_account.insert(message_id);
                let message =
                    ProxiedMessage::new(Payload::Data(bytes.to_vec()), session_id, message_id);
                let coded_message = bincode::serialize(&message)?;
                sender
                    .send_message(server_addr, &coded_message, IncludedSurbs::Amount(100))
                    .await?;
                info!(
                    "Sent message with id {} for session {} of {} bytes",
                    message_id,
                    session_id,
                    bytes.len()
                );
            }
            message_id += 1;
            let message = ProxiedMessage::new(Payload::Close, session_id, message_id);

            let coded_message = bincode::serialize(&message)?;
            sender
                .send_message(server_addr, &coded_message, IncludedSurbs::Amount(100))
                .await?;

            info!(":: Closing read end of session: {}", session_id);
            tx.send(true)
                .map_err(|_| anyhow::anyhow!("Could not send close signal"))?;
            Ok::<(), anyhow::Error>(())
        });

        // 'Incoming' thread
        tokio::spawn(async move {
            // Abstraction containing logic ordering: all our incoming messages need to be parsed based on their messageIDs per session.
            // All the message-ordering and time-tracking methods are defined in utils.rs, mostly used in .tick().
            let mut msg_buffer = MessageBuffer::new();
            // Select!-ing one of following options:
            // - rx is triggered by tx to log the session will end in ARGS.close_timeout time, break from this loop to pass to loop below
            // - Deserialise incoming mixnet message, push to msg buffer and tick() to order and write to OwnedWriteHalf.
            // - call tick() once per 100ms if neither of the above have occurred.
            loop {
                tokio::select! {
                    _ = &mut rx => {
                        info!(":: Closing write end of session: {} in {} seconds", session_id, close_timeout);
                        break
                    }
                    Some(message) = client.next() => {
                        let message = bincode::deserialize::<ProxiedMessage>(&message.message)?;
                        msg_buffer.push(message);
                        msg_buffer.tick(&mut write).await?;
                    },
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        msg_buffer.tick(&mut write).await?;
                    }
                }
            }
            // Select!-ing one of following options:
            // - Deserialise incoming mixnet message, push to msg buffer and tick() to order and write next messageID in line to OwnedWriteHalf.
            // - Sleep for session timeout and return, kills thread with Ok(()).
            loop {
                tokio::select! {
                    Some(message) = client.next() => {
                        let message = bincode::deserialize::<ProxiedMessage>(&message.message)?;
                        msg_buffer.push(message);
                        msg_buffer.tick(&mut write).await?;
                    },
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(close_timeout)) => {
                        info!(":: Closing write end of session: {}", session_id);
                        info!(":: Triggering client shutdown");
                        client.disconnect().await;
                        return Ok::<(), anyhow::Error>(())
                    }
                }
            }
        });
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}
