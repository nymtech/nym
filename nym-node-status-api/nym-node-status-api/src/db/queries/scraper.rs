use crate::db::{models::ScraperNodeInfo, DbPool};
use anyhow::Result;

pub(crate) async fn fetch_active_nodes(pool: &DbPool) -> Result<Vec<ScraperNodeInfo>> {
    let mut conn = pool.acquire().await?;
    let nodes = sqlx::query_as!(
        ScraperNodeInfo,
        r#"
            SELECT mix_id as node_id, host, http_api_port
            FROM mixnodes
            WHERE bonded = true
        "#
    )
    .fetch_all(&mut *conn)
    .await?;

    // TODO dz join this with nym node info
    Ok(nodes)
}

// TODO: add stuff for gateways
