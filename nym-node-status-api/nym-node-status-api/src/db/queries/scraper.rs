use crate::{
    db::{
        models::{ScrapeNodeKind, ScraperNodeInfo},
        queries::{
            self, gateways::insert_gateway_description, mixnodes::insert_mixnode_description,
            nym_nodes::insert_nym_node_description,
        },
        DbPool,
    },
    node_scraper::helpers::NodeDescriptionResponse,
    utils::now_utc,
};
use anyhow::Result;
use nym_validator_client::nym_api::SkimmedNode;

pub(crate) async fn get_nodes_for_scraping(pool: &DbPool) -> Result<Vec<ScraperNodeInfo>> {
    let mut nodes_to_scrape = Vec::new();

    let mixnode_ids = queries::get_bonded_mix_ids(pool).await?;
    let gateway_keys = queries::get_bonded_gateway_id_keys(pool).await?;

    let mut entry_exit_nodes = 0;
    let skimmed_nodes = queries::get_described_bonded_nym_nodes(pool)
        .await
        .map(|nodes_dto| {
            nodes_dto.into_iter().filter_map(|node| {
                let node_id = node.node_id;
                match SkimmedNode::try_from(node) {
                    Ok(node) => Some(node),
                    Err(e) => {
                        tracing::error!("Failed to decode node_id={}: {}", node_id, e);
                        None
                    }
                }
            })
        })?;

    skimmed_nodes.for_each(|node| {
        // TODO: relies on polyfilling: Nym nodes table might contain legacy mixnodes
        // as well. Categorize them here.
        let node_kind = if mixnode_ids.contains(&node.node_id.into()) {
            ScrapeNodeKind::LegacyMixnode {
                mix_id: node.node_id.into(),
            }
        } else if gateway_keys.contains(&node.ed25519_identity_pubkey.to_base58_string()) {
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
            http_api_port: node.mix_port.into(),
        })
    });

    tracing::debug!("Fetched {} 🌟 total nym nodes", nodes_to_scrape.len());
    tracing::debug!("Fetched {} 🚪 entry/exit nodes", entry_exit_nodes);

    let mut conn = pool.acquire().await?;
    let mixnodes = sqlx::query!(
        r#"
            SELECT mix_id as node_id, host, http_api_port
            FROM mixnodes
            WHERE bonded = true
        "#
    )
    .fetch_all(&mut *conn)
    .await?;
    drop(conn);

    tracing::debug!("Fetched {} 🦖 mixnodes", mixnodes.len());

    let mut duplicates = 0;
    let mut legacy_not_in_nym_node_list = 0;
    let total_legacy_mixnodes = mixnodes.len();
    for mixnode in mixnodes {
        if nodes_to_scrape
            .iter()
            .all(|node| node.node_id() != &mixnode.node_id)
        {
            // in case polyfilling on Nym API gets removed, this part ensures
            // mixnodes are added to the final list of nodes to scrape
            nodes_to_scrape.push(ScraperNodeInfo {
                node_kind: ScrapeNodeKind::LegacyMixnode {
                    mix_id: mixnode.node_id,
                },
                hosts: vec![mixnode.host],
                http_api_port: mixnode.http_api_port,
            });

            legacy_not_in_nym_node_list += 1;
        } else {
            duplicates += 1;
        }
    }
    tracing::debug!(
        "{}/{} legacy mixnodes already included in nym_node list",
        duplicates,
        total_legacy_mixnodes
    );
    tracing::debug!(
        "{}/{} legacy mixnodes NOT included in nym_node list",
        legacy_not_in_nym_node_list,
        total_legacy_mixnodes
    );
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
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            insert_mixnode_description(&mut conn, mix_id, description, timestamp).await?;
        }
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
