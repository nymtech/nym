use crate::{
    db::{
        models::{NodeKind, ScraperNodeInfo},
        queries, DbPool,
    },
    mixnet_scraper::helpers::NodeDescriptionResponse,
};
use anyhow::Result;
use chrono::Utc;

pub(crate) async fn get_mixing_nodes_for_scraping(pool: &DbPool) -> Result<Vec<ScraperNodeInfo>> {
    let mut nodes_to_scrape = Vec::new();

    queries::get_nym_nodes(pool)
        .await?
        .into_iter()
        .for_each(|node| {
            nodes_to_scrape.push(ScraperNodeInfo {
                node_id: node.node_id.into(),
                node_kind: NodeKind::NymNode,
                hosts: node
                    .ip_addresses
                    .into_iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>(),
                http_api_port: node.mix_port.into(),
            })
        });

    tracing::debug!("Fetched {} 🌟 nym nodes", nodes_to_scrape.len());

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

    tracing::debug!("Fetched {} 🦖 mixnodes", nodes_to_scrape.len());

    let mut duplicates = 0;
    let mut legacy_not_in_nym_node_list = 0;
    let total_legacy_mixnodes = mixnodes.len();
    for mixnode in mixnodes {
        if nodes_to_scrape
            .iter()
            .all(|node| node.node_id != mixnode.node_id)
        {
            legacy_not_in_nym_node_list += 1;
        } else {
            duplicates += 1;
        }

        // technically, mixnodes shouldn't be in nym_nodes table, but it's
        // possible due to polyfilling on Nym API
        if nodes_to_scrape
            .iter()
            .all(|node| node.node_id != mixnode.node_id)
        {
            nodes_to_scrape.push(ScraperNodeInfo {
                node_id: mixnode.node_id,
                node_kind: NodeKind::LegacyMixnode,
                hosts: vec![mixnode.host],
                http_api_port: mixnode.http_api_port,
            })
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

// TODO: add stuff for gateways

pub(crate) async fn insert_scraped_node_description(
    pool: &DbPool,
    node_kind: &NodeKind,
    node_id: i64,
    description: &NodeDescriptionResponse,
) -> Result<()> {
    let timestamp = Utc::now().timestamp();
    let mut conn = pool.acquire().await?;

    match node_kind {
        NodeKind::LegacyMixnode => {
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
        NodeKind::NymNode => {
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
    }

    Ok(())
}
