//! Network requester selection and directory-based auto-discovery for the
//! SOCKS5 client.

use std::collections::HashMap;

use celes::Country;
use rand::seq::SliceRandom;
use tracing::{debug, info, warn};

use nym_crypto::asymmetric::ed25519;
use nym_sphinx::addressing::clients::Recipient;
use nym_validator_client::nym_api::NymApiClientExt;

use crate::ip_packet_client::discovery::create_nym_api_client;
use crate::{Error, NymNetworkDetails};

/// Choose which network requester (the exit service that makes requests on the
/// client's behalf) a SOCKS5 client routes through. Three ways, increasing specificity.
#[derive(Debug, Clone, Default)]
pub enum NetworkRequesterSelector {
    /// Auto-discover one from the current topology, weighted by performance. (default)
    #[default]
    Any,
    /// Auto-discover, restricted to requesters physically located in one of
    /// these ISO 3166 alpha-2 countries (e.g. `["CH", "DE"]`).
    InCountries(Vec<Country>),
    /// A specific requester address you already know.
    Exact(Box<Recipient>),
}

impl NetworkRequesterSelector {
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

        // An empty filter would silently resolve as "any country". Reject it so
        // the mistake surfaces here rather than as a surprising any-country pick.
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
            Self::Any => discover(&[]).await,
            // Variants are pub, so InCountries(vec![]) can bypass in_countries()'s
            // guard; re-check here or the filter silently degrades to any-country.
            Self::InCountries(countries) if countries.is_empty() => {
                Err(Error::NoCountriesSpecified)
            }
            Self::InCountries(countries) => discover(countries).await,
        }
    }
}

/// Query the mainnet directory for network requesters and pick one weighted by
/// performance, optionally restricted to `countries` (empty slice = any).
///
/// Mirrors the IPR discovery in [`crate::ip_packet_client::discovery`]: the same
/// described-node payload carries both, so this reads `network_requester` where
/// that reads `ip_packet_router`, and location rides along for country filtering.
async fn discover(countries: &[Country]) -> Result<Recipient, Error> {
    let nym_api_urls = NymNetworkDetails::new_mainnet().nym_api_urls();

    if nym_api_urls.is_empty() {
        return Err(Error::NoNymAPIUrl);
    }

    let client = create_nym_api_client(nym_api_urls)?;
    get_best_network_requester_in(client, countries).await
}

/// A network requester on an exit gateway and the metadata the directory reports for it.
struct NetworkRequesterWithPerformance {
    address: Recipient,
    identity: ed25519::PublicKey,
    performance: u8,
    /// Physical location the operator self-reported, if any. `None` means the
    /// operator did not declare one, not that the node has no location.
    country: Option<Country>,
}

/// Collect every node advertising a network requester, with its performance
/// score and self-reported country.
async fn retrieve_network_requesters_with_performance(
    client: nym_http_api_client::Client,
) -> Result<Vec<NetworkRequesterWithPerformance>, Error> {
    let all_nodes = client
        .get_all_described_nodes_v2()
        .await?
        .into_iter()
        .map(|described| (described.ed25519_identity_key(), described))
        .collect::<HashMap<_, _>>();

    let basic_nodes = client.get_all_basic_nodes_with_metadata().await?.nodes;

    let mut requesters = Vec::new();

    for node_meta in basic_nodes {
        let Some(node) = all_nodes.get(&node_meta.ed25519_identity_pubkey) else {
            // The described set is a scraped subset of the basic set, so a basic
            // node may lack a described record (recently bonded, or unreachable
            // for scraping). Common and not actionable, so debug not warn.
            debug!(
                "{} has no described-node record; skipping",
                node_meta.ed25519_identity_pubkey
            );
            continue;
        };

        let Some(nr_info) = node.description.network_requester.clone() else {
            continue;
        };

        match nr_info.address.parse() {
            Ok(parsed_address) => requesters.push(NetworkRequesterWithPerformance {
                address: parsed_address,
                identity: node_meta.ed25519_identity_pubkey,
                performance: node_meta.performance.round_to_integer(),
                country: node.description.auxiliary_details.location,
            }),
            // A node advertising a requester with an unparseable address is
            // malformed metadata. Drop it, but say which node and why rather
            // than shrinking the pool silently.
            Err(err) => warn!(
                "{} advertises an unparseable network requester address {:?}: {err}; skipping",
                node_meta.ed25519_identity_pubkey, nr_info.address
            ),
        }
    }

    Ok(requesters)
}

/// Select a network requester weighted by performance, restricted to `countries`
/// (empty = any). Requesters with no declared location are excluded when a filter
/// is active (an undeclared node can't be assumed to match). Returns
/// [`Error::NoGatewayInCountries`] if the filter leaves none.
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

    // Weight by performance. If every candidate rounds to 0, fall back to a
    // uniform pick rather than failing. The pool is non-empty here.
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
