use crate::{
    db::{
        DbPool,
        models::{ScrapeNodeKind, ScraperNodeInfo},
        queries::{
            self, gateways::insert_gateway_description, nym_nodes::insert_nym_node_description,
        },
    },
    node_scraper::helpers::NodeDescriptionResponse,
    utils::now_utc,
};
use anyhow::Result;
use nym_validator_client::nym_api::SkimmedNode;

pub(crate) async fn get_nodes_for_scraping(pool: &DbPool) -> Result<Vec<ScraperNodeInfo>> {
    let mut nodes_to_scrape = Vec::new();

    let gateway_keys = queries::get_bonded_gateway_id_keys(pool).await?;

    let mut entry_exit_nodes = 0;
    let skimmed_nodes = queries::get_described_bonded_nym_nodes(pool)
        .await
        .map(|nodes_dto| {
            nodes_dto.into_iter().filter_map(|node_dto| {
                let node_id = node_dto.node_id;
                let http_api_port = node_dto.http_api_port;
                match SkimmedNode::try_from(node_dto) {
                    Ok(node) => Some((node, http_api_port)),
                    Err(e) => {
                        tracing::error!("Failed to decode node_id={}: {}", node_id, e);
                        None
                    }
                }
            })
        })?;

    skimmed_nodes.for_each(|(node, http_api_port)| {
        // TODO: relies on polyfilling: Nym nodes table might contain legacy mixnodes
        // as well. Categorize them here.
        let node_kind = if gateway_keys.contains(&node.ed25519_identity_pubkey.to_base58_string()) {
            entry_exit_nodes += 1;
            ScrapeNodeKind::EntryExitNymNode {
                node_id: node.node_id.into(),
                identity_key: node.ed25519_identity_pubkey.to_base58_string(),
            }
        } else {
            ScrapeNodeKind::MixingNymNode {
                node_id: node.node_id.into(),
            }
        };
        nodes_to_scrape.push(ScraperNodeInfo {
            node_kind,
            hosts: node
                .ip_addresses
                .into_iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>(),
            http_api_port: http_api_port.map(|port| port as u16),
        })
    });

    tracing::debug!("Fetched {} 🌟 total nym nodes", nodes_to_scrape.len());
    tracing::debug!("Fetched {} 🚪 entry/exit nodes", entry_exit_nodes);
    tracing::debug!("In total: {} 🌟+🦖 mixing nodes", nodes_to_scrape.len());

    Ok(nodes_to_scrape)
}

pub(crate) async fn insert_scraped_node_description(
    pool: &DbPool,
    node_kind: &ScrapeNodeKind,
    description: &NodeDescriptionResponse,
) -> Result<()> {
    let timestamp = now_utc().unix_timestamp();
    let mut conn = pool.acquire().await?;

    match node_kind {
        ScrapeNodeKind::MixingNymNode { node_id } => {
            insert_nym_node_description(&mut conn, node_id, description, timestamp).await?;
        }
        ScrapeNodeKind::EntryExitNymNode {
            node_id,
            identity_key,
        } => {
            insert_nym_node_description(&mut conn, node_id, description, timestamp).await?;
            // for historic reasons (/gateways API), store this info into gateways table as well
            insert_gateway_description(&mut conn, identity_key, description, timestamp).await?;
        }
    }

    Ok(())
}
