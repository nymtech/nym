use std::net::SocketAddr;
use std::time::Duration;

use tokio::sync::RwLockReadGuard;

use nym_client_core::client::base_client::ClientState;
use nym_socks5_client_core::config::Socks5;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::LaneQueueLengths;
use nym_task::ShutdownTracker;
use nym_topology::{NymRouteProvider, NymTopology, NymTopologyError};

use crate::mixnet::client::MixnetClientBuilder;
use crate::mixnet::NetworkRequesterSelector;
use crate::Result;

/// A SOCKS5 proxy client connected to the Nym mixnet.
///
/// `Socks5MixnetClient` provides a SOCKS5 proxy interface to the Nym mixnet,
/// allowing HTTP(S) clients and other SOCKS5-compatible applications to route
/// their traffic through the mixnet without having to modify their networking
/// code.
///
/// Traffic leaves the mixnet through a network requester: a service running on
/// an exit gateway that makes requests on the client's behalf and enforces the
/// Nym exit policy. You can let the client discover one for you or name a specific
/// one; see [`connect_with`](Self::connect_with) and [`NetworkRequesterSelector`].
///
/// ## Usage
///
/// 1. Connect, either by discovering a requester with
///    [`connect_with`](Self::connect_with) or naming a known one with
///    [`connect_new`](Self::connect_new)
/// 2. Get the SOCKS5 URL via [`socks5_url`](Self::socks5_url)
/// 3. Point your HTTP client at that SOCKS5 proxy
///
/// ## Example
///
/// ```rust,no_run
/// use nym_sdk::mixnet::Socks5MixnetClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Connect to a known network requester by address
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
// ```
pub struct Socks5MixnetClient {
    /// The nym address of this connected client.
    pub(crate) nym_address: Recipient,

    /// The current state of the client that is exposed to the user. This includes things like
    /// current message send queue length.
    pub(crate) client_state: ClientState,

    /// The task manager controlling all the spawned tasks the client uses to do its job.
    pub(crate) task_handle: ShutdownTracker,

    /// SOCKS5 configuration parameters.
    pub(crate) socks5_config: Socks5,
}

impl Socks5MixnetClient {
    /// Create a new client and connect to a network requester over the mixnet via SOCKS5 using
    /// ephemeral in-memory keys that are discarded at application close.
    ///
    /// This is the zero-ceremony path when you already know the requester's
    /// address; it is shorthand for [`connect_with`](Self::connect_with) with
    /// [`NetworkRequesterSelector::exact`] and the default listener bind.
    ///
    /// Kept for backwards compatibility: it predates [`connect_with`] and overlaps
    /// with the `exact` case, but existing callers pass an address string directly.
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

    /// Create a new client and connect to a network requester chosen per the
    /// given [`NetworkRequesterSelector`]: auto-discovered ([`Any`](NetworkRequesterSelector::Any)),
    /// country-restricted ([`InCountries`](NetworkRequesterSelector::InCountries)), or a
    /// known address ([`Exact`](NetworkRequesterSelector::Exact)).
    ///
    /// The discovered requester enforces the Nym exit policy, so destinations
    /// outside that policy are refused at the exit regardless of which
    /// requester is selected.
    ///
    /// `bind` sets the local SOCKS5 listener address; pass `None` for the default
    /// `127.0.0.1:1080`, or `Some(addr)` to move it (for example when 1080 is
    /// already taken, or to run more than one client at once).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{NetworkRequesterSelector, Socks5MixnetClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Any requester, weighted by performance, on the default port:
    ///     let any = Socks5MixnetClient::connect_with(NetworkRequesterSelector::any(), None).await?;
    ///
    ///     // Pinned to Switzerland or Germany, listening on 127.0.0.1:1081:
    ///     let pinned = Socks5MixnetClient::connect_with(
    ///         NetworkRequesterSelector::in_countries(["CH", "DE"])?,
    ///         Some("127.0.0.1:1081".parse()?),
    ///     )
    ///     .await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect_with(
        requester: NetworkRequesterSelector,
        bind: Option<SocketAddr>,
    ) -> Result<Self> {
        let provider = requester.resolve().await?;
        let mut socks5_config = Socks5::new(provider.to_string());
        if let Some(addr) = bind {
            socks5_config.bind_address = addr;
        }
        MixnetClientBuilder::new_ephemeral()
            .socks5_config(socks5_config)
            .build()?
            .connect_to_mixnet_via_socks5()
            .await
    }

    /// Get the nym address of this client. The nym address is composed of the
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
