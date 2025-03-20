use crate::{
    db::{
        models::{ScrapeNodeKind, ScraperNodeInfo},
        queries, DbPool,
    },
    scrapers::node_info::helpers::NodeDescriptionResponse,
};
use anyhow::Result;
use chrono::Utc;
use nym_validator_client::nym_api::SkimmedNode;

pub(crate) async fn get_nodes_for_scraping(pool: &DbPool) -> Result<Vec<ScraperNodeInfo>> {
    let mut nodes_to_scrape = Vec::new();

    let mixnode_ids = queries::get_bonded_mix_ids(pool).await?;
    let gateway_keys = queries::get_bonded_gateway_id_keys(pool).await?;

    let mut entry_exit_nodes = 0;
    let skimmed_nodes = queries::get_active_nym_nodes(pool).await.map(|nodes_dto| {
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
    let timestamp = Utc::now().timestamp();
    let mut conn = pool.acquire().await?;

    match node_kind {
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            sqlx::query!(
                r#"
                INSERT INTO mixnode_description (
                    mix_id, moniker, website, security_contact, details, last_updated_utc
                ) VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (mix_id) DO UPDATE SET
                    moniker = excluded.moniker,
                    website = excluded.website,
                    security_contact = excluded.security_contact,
                    details = excluded.details,
                    last_updated_utc = excluded.last_updated_utc
                "#,
                mix_id,
                description.moniker,
                description.website,
                description.security_contact,
                description.details,
                timestamp,
            )
            .execute(&mut *conn)
            .await?;
        }
        ScrapeNodeKind::MixingNymNode { node_id } => {
            sqlx::query!(
                r#"
                INSERT INTO nym_node_descriptions (
                    node_id, moniker, website, security_contact, details, last_updated_utc
                ) VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (node_id) DO UPDATE SET
                    moniker = excluded.moniker,
                    website = excluded.website,
                    security_contact = excluded.security_contact,
                    details = excluded.details,
                    last_updated_utc = excluded.last_updated_utc
                "#,
                node_id,
                description.moniker,
                description.website,
                description.security_contact,
                description.details,
                timestamp,
            )
            .execute(&mut *conn)
            .await?;
        }
        ScrapeNodeKind::EntryExitNymNode { identity_key, .. } => {
            sqlx::query!(
                r#"
            INSERT INTO gateway_description (
                gateway_identity_key,
                moniker,
                website,
                security_contact,
                details,
                last_updated_utc
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (gateway_identity_key) DO UPDATE SET
                moniker = excluded.moniker,
                website = excluded.website,
                security_contact = excluded.security_contact,
                details = excluded.details,
                last_updated_utc = excluded.last_updated_utc
            "#,
                identity_key,
                description.moniker,
                description.website,
                description.security_contact,
                description.details,
                timestamp,
            )
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(())
}
