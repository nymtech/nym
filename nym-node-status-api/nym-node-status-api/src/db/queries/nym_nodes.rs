use std::collections::HashMap;

use anyhow::Context;
use futures_util::TryStreamExt;
use nym_validator_client::{client::NymNodeDetails, nym_api::SkimmedNode};
use tracing::instrument;

use crate::{
    db::{
        models::{NymNodeDto, NymNodeInsertRecord},
        DbPool,
    },
    utils::decimal_to_i64,
};

pub(crate) async fn get_nym_nodes(pool: &DbPool) -> anyhow::Result<Vec<SkimmedNode>> {
    let mut conn = pool.acquire().await?;

    let items = sqlx::query_as!(
        NymNodeDto,
        r#"SELECT
            node_id,
            ed25519_identity_pubkey,
            total_stake,
            ip_addresses as "ip_addresses!: serde_json::Value",
            mix_port,
            x25519_sphinx_pubkey,
            node_role as "node_role: serde_json::Value",
            supported_roles as "supported_roles: serde_json::Value",
            entry as "entry: serde_json::Value",
            performance
        FROM
            nym_nodes
        "#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<NymNodeDto>>()
    .await?;

    let mut skimmed_nodes = Vec::new();
    for item in items {
        let node_id = item.node_id;
        match SkimmedNode::try_from(item) {
            Ok(node) => skimmed_nodes.push(node),
            Err(e) => {
                tracing::warn!("Failed to decode node_id={}: {}", node_id, e);
            }
        }
    }

    Ok(skimmed_nodes)
}

#[instrument(level = "debug", skip_all)]
pub(crate) async fn insert_nym_nodes(
    pool: &DbPool,
    nym_nodes: Vec<SkimmedNode>,
    bonded_node_info: &HashMap<u32, NymNodeDetails>,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    for nym_node in nym_nodes.into_iter() {
        let total_stake = bonded_node_info
            .get(&nym_node.node_id)
            .map(|details| decimal_to_i64(details.total_stake()))
            .unwrap_or(0);

        let record = NymNodeInsertRecord::new(nym_node, total_stake)?;
        // https://www.sqlite.org/lang_upsert.html
        sqlx::query!(
            "INSERT INTO nym_nodes
                (node_id, ed25519_identity_pubkey,
                    total_stake,
                    ip_addresses, mix_port,
                    x25519_sphinx_pubkey, node_role,
                    supported_roles, entry,
                    performance, last_updated_utc
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(node_id) DO UPDATE SET
                ed25519_identity_pubkey=excluded.ed25519_identity_pubkey,
                ip_addresses=excluded.ip_addresses,
                mix_port=excluded.mix_port,
                x25519_sphinx_pubkey=excluded.x25519_sphinx_pubkey,
                node_role=excluded.node_role,
                supported_roles=excluded.supported_roles,
                entry=excluded.entry,
                performance=excluded.performance,
                last_updated_utc=excluded.last_updated_utc
                ;",
            record.node_id,
            record.ed25519_identity_pubkey,
            record.total_stake,
            record.ip_addresses,
            record.mix_port,
            record.x25519_sphinx_pubkey,
            record.node_role,
            record.supported_roles,
            record.entry,
            record.performance,
            record.last_updated_utc,
        )
        .execute(&mut *conn)
        .await
        .with_context(|| format!("node_id={}", record.node_id))?;
    }

    Ok(())
}
