// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::helpers::load_ed25519_identity_keypair;
use anyhow::{Context, bail};
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::ed25519;
use nym_geolocator_requests::client::GeolocatorClient;
use nym_geolocator_requests::models::SignedCheckRequest;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NodeId;
use url::Url;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    /// Url of the geolocator agent to ask, for example `https://geolocator.nymtech.net`.
    #[clap(long, env = "NYM_NODE_GEOLOCATOR_URL")]
    geolocator: Url,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

/// Resolve this node's contract id from the identity key it holds.
///
/// The agent verifies the request against the key bonded under the id the request names, so the id
/// and the key have to belong to the same node. Asking the operator for it would make a typo look
/// like a forged signature; the chain already knows the answer, and a node that cannot be found
/// there could not have been measured anyway.
async fn resolve_node_id(
    nyxd_urls: &[Url],
    identity_key: &ed25519::PublicKey,
) -> anyhow::Result<NodeId> {
    let identity = identity_key.to_base58_string();

    for nyxd_url in nyxd_urls {
        let client = match QueryHttpRpcNyxdClient::connect_to_default_env(nyxd_url.as_str()) {
            Ok(client) => client,
            Err(err) => {
                eprintln!("could not use {nyxd_url}: {err}");
                continue;
            }
        };

        match client
            .get_nymnode_details_by_identity(identity.clone())
            .await
        {
            Ok(response) => {
                let details = response.details.with_context(|| {
                    format!("no bonded nym-node has the identity key {identity}")
                })?;
                return Ok(details.node_id());
            }
            Err(err) => eprintln!("failed to query {nyxd_url}: {err}"),
        }
    }

    bail!("none of the configured nyxd endpoints could be queried")
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    let config = try_load_current_config(args.config.config_path()).await?;
    let identity_keypair =
        load_ed25519_identity_keypair(&config.storage_paths.keys.ed25519_identity_storage_paths())?;

    let node_id = resolve_node_id(&config.nyx.nyxd_urls, identity_keypair.public_key()).await?;

    eprintln!(
        "asking {} to re-measure the location of node {node_id}...",
        args.geolocator
    );

    // the signature covers the node id, so the agent checks it against the key bonded under that
    // id: a request naming another operator's node cannot be made to verify
    let request = SignedCheckRequest::new(node_id, identity_keypair.private_key());

    let client = GeolocatorClient::new(args.geolocator.as_str())?;
    let response = client.request_check(&request).await?;

    args.output.to_stdout(&response);
    Ok(())
}
