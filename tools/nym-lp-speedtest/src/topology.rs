//! Topology fetching from nym-api
//!
//! Queries nym-api for active mix nodes and gateways,
//! builds routes for Sphinx packet construction.

use anyhow::Result;
use url::Url;

/// Fetch network topology from nym-api
pub async fn fetch_topology(_nym_api: &Url) -> Result<()> {
    // TODO: Implement
    // 1. Query /v1/mixnodes/active for mix nodes
    // 2. Query /v1/gateways/described for gateways
    // 3. Build TopologyNode structures
    // 4. Select random 4-hop route
    Ok(())
}
