use std::net::SocketAddr;
use std::time::Duration;

use nym_client_core::client::base_client::ClientState;
use nym_socks5_client_core::config::Socks5;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::LaneQueueLengths;
use nym_task::ShutdownTracker;
use nym_topology::{NymRouteProvider, NymTopology, NymTopologyError};

use crate::mixnet::client::MixnetClientBuilder;
use crate::{Error, Result};

use celes::Country;
use tokio::sync::RwLockReadGuard;

/// A SOCKS5 proxy client connected to the Nym mixnet.
///
/// `Socks5MixnetClient` provides a SOCKS5 proxy interface to the Nym mixnet,
/// allowing HTTP(S) clients and other SOCKS5-compatible applications to route
/// their traffic through the mixnet without having to modify their networking
/// code.
///
/// Traffic leaves the mixnet through a network requester: a service running on
/// an Exit Gateway that makes requests on the client's behalf and enforces the
/// Nym exit policy. You can let the client discover one for you or name a specific
/// one; see [`connect_with`](Self::connect_with) and [`NetworkRequester`].
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
    /// [`NetworkRequester::exact`] and the default listener bind.
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
    /// given [`NetworkRequester`]: auto-discovered ([`Any`](NetworkRequester::Any)),
    /// country-restricted ([`InCountries`](NetworkRequester::InCountries)), or a
    /// known address ([`Exact`](NetworkRequester::Exact)).
    ///
    /// The discovered requester enforces the Nym exit policy, so destinations
    /// outside that policy are refused at the  exit regardless of which
    /// requester is selected.
    ///
    /// `bind` sets the local SOCKS5 listener address; pass `None` for the default
    /// `127.0.0.1:1080`, or `Some(addr)` to move it (for example when 1080 is
    /// already taken, or to run more than one client at once).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet::{NetworkRequester, Socks5MixnetClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Any requester, weighted by performance, on the default port:
    ///     let any = Socks5MixnetClient::connect_with(NetworkRequester::any(), None).await?;
    ///
    ///     // Pinned to Switzerland or Germany, listening on 127.0.0.1:1081:
    ///     let pinned = Socks5MixnetClient::connect_with(
    ///         NetworkRequester::in_countries(["CH", "DE"])?,
    ///         Some("127.0.0.1:1081".parse()?),
    ///     )
    ///     .await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect_with(
        requester: NetworkRequester,
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

/// Which network requester (the mixnet exit that makes requests on the client's
/// behalf) a SOCKS5 client routes through. Three ways, increasing specificity.
#[derive(Debug, Clone, Default)]
pub enum NetworkRequester {
    /// Auto-discover one from the current topology, weighted by performance. (default)
    #[default]
    Any,
    /// Auto-discover, restricted to requesters physically located in one of
    /// these ISO 3166 alpha-2 countries (e.g. `["CH", "DE"]`).
    InCountries(Vec<Country>),
    /// A specific requester address you already know.
    Exact(Box<Recipient>),
}

impl NetworkRequester {
    /// Any requester, weighted by performance.
    pub fn any() -> Self {
        Self::Any
    }

    /// Restrict discovery to the given ISO 3166 alpha-2 country codes.
    /// Case-insensitive. Returns [`Error::InvalidCountryCode`] on the first
    /// code that is not a valid alpha-2 code, or [`Error::NoCountriesSpecified`]
    /// if the list is empty (use [`any`](Self::any) to accept any country).
    #[allow(clippy::result_large_err)]
    pub fn in_countries<I, S>(codes: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let countries = codes
            .into_iter()
            .map(|c| {
                Country::from_alpha2(c.as_ref())
                    .map_err(|_| Error::InvalidCountryCode(c.as_ref().to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // An empty filter would resolve as "any country", silently ignoring the
        // caller's intent to scope by location. Reject it so the mistake surfaces
        // at the call site rather than as a surprising any-country pick.
        if countries.is_empty() {
            return Err(Error::NoCountriesSpecified);
        }

        Ok(Self::InCountries(countries))
    }

    /// A specific requester by its Nym address. Returns
    /// [`Error::InvalidRecipientAddress`] if the address does not parse.
    #[allow(clippy::result_large_err)]
    pub fn exact(address: impl AsRef<str>) -> Result<Self, Error> {
        let recipient = address
            .as_ref()
            .parse()
            .map_err(|_| Error::InvalidRecipientAddress(address.as_ref().to_string()))?;
        Ok(Self::Exact(Box::new(recipient)))
    }

    /// Resolve to a concrete requester address. `Exact` returns its address
    /// directly; `Any` / `InCountries` query the mainnet directory and pick one
    /// weighted by performance.
    pub async fn resolve(&self) -> Result<Recipient, Error> {
        match self {
            Self::Exact(addr) => Ok(**addr),
            Self::Any => discovery::discover(&[]).await,
            Self::InCountries(countries) => discovery::discover(countries).await,
        }
    }
}

/// Directory crawl backing [`NetworkRequester`] auto-discovery.
///
/// This mirrors the IPR discovery in [`crate::ip_packet_client::discovery`]:
/// exit gateways self-announce both an IPR and a network requester in the same
/// described-node payload (`NymNodeDataV2`), so the selection logic matches; only
/// the field we read differs (`network_requester` instead of `ip_packet_router`),
/// and there is no protocol-version gate. The node's physical location
/// (`auxiliary_details.location`) rides along in that same payload, so country
/// filtering needs no extra requests.
mod discovery {
    use std::collections::HashMap;

    use celes::Country;
    use nym_crypto::asymmetric::ed25519;
    use nym_sphinx::addressing::clients::Recipient;
    use nym_validator_client::nym_api::NymApiClientExt;
    use rand::seq::SliceRandom;
    use tracing::{debug, info, warn};

    use crate::ip_packet_client::discovery::create_nym_api_client;
    use crate::{Error, NymNetworkDetails};

    /// Query the mainnet directory for network requesters and pick one weighted by
    /// performance, optionally restricted to `countries` (empty slice = any).
    pub(super) async fn discover(countries: &[Country]) -> Result<Recipient, Error> {
        let nym_api_urls = NymNetworkDetails::new_mainnet()
            .nym_api_urls
            .ok_or(Error::NoNymAPIUrl)?;
        let client = create_nym_api_client(nym_api_urls)?;
        get_best_network_requester_in(client, countries).await
    }

    /// A network requester exit gateway and the metadata the directory reports for it.
    struct NetworkRequesterWithPerformance {
        address: Recipient,
        identity: ed25519::PublicKey,
        performance: u8,
        /// Physical location the operator self-reported, if any. `None` means the
        /// operator did not declare a location, not that the node is unlocated.
        country: Option<Country>,
    }

    /// Collect every exit gateway that advertises a network requester address,
    /// paired with its performance score and self-reported country.
    async fn retrieve_network_requesters_with_performance(
        client: nym_http_api_client::Client,
    ) -> Result<Vec<NetworkRequesterWithPerformance>, Error> {
        let all_nodes = client
            .get_all_described_nodes_v2()
            .await?
            .into_iter()
            .map(|described| (described.ed25519_identity_key(), described))
            .collect::<HashMap<_, _>>();

        let exit_gateways = client.get_all_basic_nodes_with_metadata().await?.nodes;

        let mut requesters = Vec::new();

        for exit in exit_gateways {
            let Some(node) = all_nodes.get(&exit.ed25519_identity_pubkey) else {
                // The skimmed and described sets come from two separate API calls
                // and can be momentarily out of sync, so a node present in one but
                // not the other is expected churn rather than an error; log at debug.
                debug!(
                    "{} has no described-node record; skipping",
                    exit.ed25519_identity_pubkey
                );
                continue;
            };

            let Some(nr_info) = node.description.network_requester.clone() else {
                continue;
            };

            match nr_info.address.parse() {
                Ok(parsed_address) => requesters.push(NetworkRequesterWithPerformance {
                    address: parsed_address,
                    identity: exit.ed25519_identity_pubkey,
                    performance: exit.performance.round_to_integer(),
                    country: node.description.auxiliary_details.location,
                }),
                // A node that advertises a requester but with an unparseable address
                // is malformed metadata. Drop it from the pool, but say which node
                // and why rather than shrinking the pool silently.
                Err(err) => warn!(
                    "{} advertises an unparseable network requester address {:?}: {err}; skipping",
                    exit.ed25519_identity_pubkey, nr_info.address
                ),
            }
        }

        Ok(requesters)
    }

    /// Select a network requester weighted by performance, restricted to the given
    /// countries. An empty `countries` slice means any country is acceptable.
    ///
    /// Requesters that did not declare a location are excluded whenever a country
    /// filter is active: an undeclared exit cannot be assumed to be in a requested
    /// country. If the filter leaves no candidates, this returns
    /// [`Error::NoGatewayInCountries`] rather than silently falling back.
    async fn get_best_network_requester_in(
        client: nym_http_api_client::Client,
        countries: &[Country],
    ) -> Result<Recipient, Error> {
        let requesters = retrieve_network_requesters_with_performance(client).await?;
        let total = requesters.len();

        let pool: Vec<NetworkRequesterWithPerformance> = if countries.is_empty() {
            requesters
        } else {
            requesters
                .into_iter()
                .filter(|nr| match nr.country {
                    Some(c) => countries
                        .iter()
                        .any(|want| want.alpha2.eq_ignore_ascii_case(c.alpha2)),
                    None => false,
                })
                .collect()
        };

        info!(
            "Found {} network requesters ({} after country filter)",
            total,
            pool.len()
        );

        if pool.is_empty() {
            return Err(if countries.is_empty() {
                Error::NoGatewayAvailable
            } else {
                Error::NoGatewayInCountries
            });
        }

        // Weight by performance. If every candidate scored zero (e.g. a low score
        // rounded down to 0), fall back to a uniform pick rather than failing as if
        // no requester existed. The pool is non-empty here.
        let mut rng = rand::thread_rng();
        let selected = pool
            .choose_weighted(&mut rng, |nr| nr.performance as f64)
            .or_else(|_| pool.choose(&mut rng).ok_or(Error::NoGatewayAvailable))?;

        info!(
            "Using network requester: {} (Gateway: {}, Country: {:?}, Performance: {:?})",
            selected.address,
            selected.identity,
            selected.country.map(|c| c.alpha2),
            selected.performance
        );

        Ok(selected.address)
    }
}
