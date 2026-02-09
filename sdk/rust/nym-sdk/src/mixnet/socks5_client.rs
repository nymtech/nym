use std::time::Duration;

use nym_client_core::client::base_client::ClientState;
use nym_socks5_client_core::config::Socks5;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::LaneQueueLengths;
use nym_task::ShutdownTracker;
use tokio::sync::RwLockReadGuard;

use nym_topology::{NymRouteProvider, NymTopology, NymTopologyError};

use crate::mixnet::client::MixnetClientBuilder;
use crate::Result;

/// A SOCKS5 proxy client connected to the Nym mixnet.
///
/// `Socks5MixnetClient` provides a SOCKS5 proxy interface to the Nym mixnet,
/// allowing HTTP(S) clients and other SOCKS5-compatible applications to route
/// their traffic through the mixnet for enhanced privacy.
///
/// ## Usage
///
/// 1. Connect to a service provider via [`connect_new`](Self::connect_new)
/// 2. Get the SOCKS5 URL via [`socks5_url`](Self::socks5_url)
/// 3. Configure your HTTP client to use this SOCKS5 proxy
///
/// ## Example
///
/// ```rust,no_run
/// use nym_sdk::mixnet::Socks5MixnetClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Connect to a network requester service provider
///     let client = Socks5MixnetClient::connect_new("provider_nym_address...").await?;
///
///     // Get the SOCKS5 proxy URL
///     let socks5_url = client.socks5_url();
///     println!("Configure your HTTP client to use: {}", socks5_url);
///
///     // Your HTTP client can now use the SOCKS5 proxy
///     // let http_client = reqwest::Client::builder()
///     //     .proxy(reqwest::Proxy::all(&socks5_url)?)
///     //     .build()?;
///
///     client.disconnect().await;
///     Ok(())
/// }
/// ```
///
/// ## Service Providers
///
/// The SOCKS5 client connects to a "network requester" service provider that
/// makes HTTP requests on behalf of the client. The service provider's Nym
/// address must be provided when creating the client.
pub struct Socks5MixnetClient {
    /// The nym address of this connected client.
    pub(crate) nym_address: Recipient,

    /// The current state of the client that is exposed to the user. This includes things like
    /// current message send queue length.
    pub(crate) client_state: ClientState,

    /// The task manager that controls all the spawned tasks that the clients uses to do it's job.
    pub(crate) task_handle: ShutdownTracker,

    /// SOCKS5 configuration parameters.
    pub(crate) socks5_config: Socks5,
}

impl Socks5MixnetClient {
    /// Create a new client and connect to a service provider over the mixnet via SOCKS5 using
    /// ephemeral in-memory keys that are discarded at application close.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let receiving_client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     let mut client = mixnet::Socks5MixnetClient::connect_new(receiving_client.nym_address().to_string()).await;
    /// }
    ///
    /// ```
    pub async fn connect_new<S: Into<String>>(provider_mix_address: S) -> Result<Self> {
        MixnetClientBuilder::new_ephemeral()
            .socks5_config(Socks5::new(provider_mix_address))
            .build()?
            .connect_to_mixnet_via_socks5()
            .await
    }

    /// Get the nym address for this client, if it is available. The nym address is composed of the
    /// client identity, the client encryption key, and the gateway identity.
    pub fn nym_address(&self) -> &Recipient {
        &self.nym_address
    }

    /// Get the SOCKS5 proxy URL that a HTTP(S) client can connect to.
    pub fn socks5_url(&self) -> String {
        format!("socks5h://{}", self.socks5_config.bind_address)
    }

    /// Get a shallow clone of [`LaneQueueLengths`]. This is useful to manually implement some form
    /// of backpressure logic.
    pub fn shared_lane_queue_lengths(&self) -> LaneQueueLengths {
        self.client_state.shared_lane_queue_lengths.clone()
    }

    /// Change the network topology used by this client for constructing sphinx packets into the
    /// provided one.
    pub async fn manually_overwrite_topology(&self, new_topology: NymTopology) {
        self.client_state
            .topology_accessor
            .manually_change_topology(new_topology)
            .await
    }

    /// Restore default topology refreshing behaviour of this client.
    pub fn restore_automatic_topology_refreshing(&self) {
        self.client_state.topology_accessor.release_manual_control()
    }

    /// Disconnect from the mixnet. Currently it is not supported to reconnect a disconnected
    /// client.
    pub async fn disconnect(self) {
        self.task_handle.shutdown().await;
    }

    /// Gets the current route provider if topology is available.
    /// Returns `None` if topology is empty/not yet fetched.
    async fn read_current_route_provider(&self) -> Option<RwLockReadGuard<'_, NymRouteProvider>> {
        self.client_state
            .topology_accessor
            .current_route_provider()
            .await
    }

    /// Wait for topology to become available, with a timeout.
    /// Returns `Ok(())` when topology is ready, or `Err` if timeout is reached.
    pub async fn wait_for_topology(&self, timeout: Duration) -> Result<(), NymTopologyError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.read_current_route_provider().await.is_some() {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(NymTopologyError::EmptyNetworkTopology);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
