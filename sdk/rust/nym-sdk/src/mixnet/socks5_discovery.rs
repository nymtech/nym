// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Choosing the network requester (the mixnet exit) a SOCKS5 client routes
//! through, and the directory query that backs auto-discovery.
//!
//! [`NetworkRequester`] is the public face: `Any`, `InCountries`, or `Exact`.
//! The directory crawl and performance weighting below are private detail that
//! [`NetworkRequester::resolve`] drives.
//!
//! Discovery mirrors the IPR discovery in [`crate::ip_packet_client::discovery`]:
//! exit gateways self-announce both an IPR and a network requester in the same
//! described-node payload (`NymNodeDataV2`), so the selection logic matches; only
//! the field we read differs (`network_requester` instead of `ip_packet_router`),
//! and there is no protocol-version gate. The node's physical location
//! (`auxiliary_details.location`) rides along in that same payload, so country
//! filtering needs no extra requests.

use std::collections::HashMap;

use celes::Country;
use nym_crypto::asymmetric::ed25519;
use nym_sphinx::addressing::clients::Recipient;
use nym_validator_client::nym_api::NymApiClientExt;
use rand::seq::SliceRandom;
use tracing::{debug, info, warn};

use crate::ip_packet_client::discovery::create_nym_api_client;
use crate::{Error, NymNetworkDetails};

/// Which network requester (the mixnet exit that makes requests on the client's
/// behalf) a SOCKS5 client routes through. Three ways, increasing specificity.
#[derive(Debug, Clone, Default)]
pub enum NetworkRequester {
    /// Auto-discover one from the directory, weighted by performance. (default)
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
            Self::Exact(addr) => Ok((**addr).clone()),
            Self::Any => discover(&[]).await,
            Self::InCountries(countries) => discover(countries).await,
        }
    }
}

/// Query the mainnet directory for network requesters and pick one weighted by
/// performance, optionally restricted to `countries` (empty slice = any).
async fn discover(countries: &[Country]) -> Result<Recipient, Error> {
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
