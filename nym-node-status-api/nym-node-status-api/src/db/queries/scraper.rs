use crate::db::{models::ScraperNodeInfo, queries, DbPool};
use anyhow::Result;

pub(crate) async fn fetch_mixing_nodes(pool: &DbPool) -> Result<Vec<ScraperNodeInfo>> {
    let mut nodes_to_scrape = Vec::new();

    queries::get_nym_nodes(pool)
        .await?
        .into_iter()
        .for_each(|node| {
            nodes_to_scrape.push(ScraperNodeInfo {
                node_id: node.node_id.into(),
                hosts: node
                    .ip_addresses
                    .into_iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>(),
                http_api_port: node.mix_port.into(),
            })
        });

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

    for mixnode in mixnodes {
        nodes_to_scrape.push(ScraperNodeInfo {
            node_id: mixnode.node_id,
            hosts: vec![mixnode.host],
            http_api_port: mixnode.http_api_port,
        })
    }

    Ok(nodes_to_scrape)
}

// TODO: add stuff for gateways
