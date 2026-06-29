use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use nym_client_core::client::base_client::ClientState;
use nym_socks5_client_core::config::Socks5;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::LaneQueueLengths;
use nym_task::ShutdownTracker;
use tokio::sync::RwLockReadGuard;

use celes::Country;
use nym_topology::{NymRouteProvider, NymTopology, NymTopologyError};

use crate::ip_packet_client::discovery::create_nym_api_client;
use crate::mixnet::client::MixnetClientBuilder;
use crate::mixnet::socks5_discovery::get_best_network_requester_in;
use crate::{Error, NymNetworkDetails, Result};

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

    /// Start building a client that connects to an automatically discovered
    /// network requester. Restrict the requester's physical location with
    /// [`country`](Socks5DiscoveryBuilder::country) /
    /// [`countries`](Socks5DiscoveryBuilder::countries), then
    /// [`connect`](Socks5DiscoveryBuilder::connect).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::Socks5MixnetClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Any country:
    ///     let any = Socks5MixnetClient::discover().connect().await?;
    ///
    ///     // Pinned to Switzerland or Germany:
    ///     let pinned = Socks5MixnetClient::discover()
    ///         .countries(["CH", "DE"])?
    ///         .connect()
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn discover() -> Socks5DiscoveryBuilder {
        Socks5DiscoveryBuilder::default()
    }

    /// Create a new client and connect to an automatically discovered network
    /// requester in any country. Shorthand for `discover().connect()`.
    ///
    /// Discovery always targets mainnet. The discovered requester enforces the
    /// Nym exit policy, so destinations outside that policy will be refused at
    /// the exit regardless of which requester is selected.
    pub async fn connect_new_with_discovery() -> Result<Self> {
        Self::discover().connect().await
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

/// Builder for connecting a [`Socks5MixnetClient`] to an automatically
/// discovered network requester, optionally restricted by country.
///
/// Create one with [`Socks5MixnetClient::discover`]. With no country set,
/// discovery selects from any country; otherwise the chosen requester must be
/// physically located in one of the requested countries.
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct Socks5DiscoveryBuilder {
    countries: Vec<Country>,
    bind_address: Option<SocketAddr>,
}

impl Socks5DiscoveryBuilder {
    /// Require the discovered network requester to be located in the given
    /// country, identified by its ISO 3166 alpha-2 code (e.g. `"CH"`).
    /// Case-insensitive. Call repeatedly to allow several countries.
    ///
    /// Returns [`Error::InvalidCountryCode`] if the code is not a valid alpha-2
    /// country code.
    #[allow(clippy::result_large_err)]
    pub fn country(mut self, code: impl AsRef<str>) -> Result<Self> {
        let country = Country::from_alpha2(code.as_ref())
            .map_err(|_| Error::InvalidCountryCode(code.as_ref().to_string()))?;
        self.countries.push(country);
        Ok(self)
    }

    /// Require the discovered network requester to be located in one of the
    /// given countries, each an ISO 3166 alpha-2 code. Case-insensitive.
    ///
    /// Returns [`Error::InvalidCountryCode`] on the first invalid code.
    #[allow(clippy::result_large_err)]
    pub fn countries<I, S>(mut self, codes: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for code in codes {
            self = self.country(code)?;
        }
        Ok(self)
    }

    /// Bind the local SOCKS5 listener to a specific address instead of the
    /// default `127.0.0.1:1080`. Set this to run more than one SOCKS5 client at
    /// once, or to avoid a port already in use.
    pub fn bind_address(mut self, address: SocketAddr) -> Self {
        self.bind_address = Some(address);
        self
    }

    /// Bind the local SOCKS5 listener to `127.0.0.1:<port>` instead of the
    /// default port 1080. Shorthand for the loopback case of
    /// [`bind_address`](Self::bind_address); this resets the host to loopback,
    /// overriding any address previously set with `bind_address`.
    pub fn port(mut self, port: u16) -> Self {
        self.bind_address = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
        self
    }

    /// Discover a matching network requester on mainnet and connect to it.
    ///
    /// If a country filter is set and no requester matches, returns
    /// [`Error::NoGatewayInCountries`].
    pub async fn connect(self) -> Result<Socks5MixnetClient> {
        let nym_api_urls = NymNetworkDetails::new_mainnet()
            .nym_api_urls
            .ok_or(Error::NoNymAPIUrl)?;
        let api_client = create_nym_api_client(nym_api_urls)?;
        let provider = get_best_network_requester_in(api_client, &self.countries).await?;

        let mut socks5_config = Socks5::new(provider.to_string());
        if let Some(bind_address) = self.bind_address {
            socks5_config.bind_address = bind_address;
        }

        MixnetClientBuilder::new_ephemeral()
            .socks5_config(socks5_config)
            .build()?
            .connect_to_mixnet_via_socks5()
            .await
    }
}
