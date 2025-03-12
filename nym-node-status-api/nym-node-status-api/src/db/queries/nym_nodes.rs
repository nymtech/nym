use std::collections::HashMap;

use anyhow::Context;
use futures_util::TryStreamExt;
use nym_validator_client::{
    client::{NodeId, NymNodeDetails},
    models::NymNodeDescription,
};
use tracing::instrument;

use crate::db::{
    models::{NymNodeDto, NymNodeInsertRecord},
    DbPool,
};

pub(crate) async fn get_all_nym_nodes(pool: &DbPool) -> anyhow::Result<Vec<NymNodeDto>> {
    let mut conn = pool.acquire().await?;

    sqlx::query_as!(
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
            performance,
            self_described as "self_described: serde_json::Value",
            bond_info as "bond_info: serde_json::Value",
            active as "active: bool"
        FROM
            nym_nodes
        "#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<NymNodeDto>>()
    .await
    .map_err(From::from)
}

pub(crate) async fn get_active_nym_nodes(pool: &DbPool) -> anyhow::Result<Vec<NymNodeDto>> {
    let mut conn = pool.acquire().await?;

    sqlx::query_as!(
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
            performance,
            self_described as "self_described: serde_json::Value",
            bond_info as "bond_info: serde_json::Value",
            active as "active: bool"
        FROM
            nym_nodes
        WHERE
            active = true
        "#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<NymNodeDto>>()
    .await
    .map_err(From::from)
}

#[instrument(level = "debug", skip_all)]
pub(crate) async fn update_nym_nodes(
    pool: &DbPool,
    node_records: Vec<NymNodeInsertRecord>,
) -> anyhow::Result<usize> {
    let mut tx = pool.begin().await?;

    // set inactive all nodes
    sqlx::query!(
        r#"UPDATE
            nym_nodes
        SET
            active = false
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // active nodes will get updated on insert
    let inserted = node_records.len();
    for record in node_records {
        // https://www.sqlite.org/lang_upsert.html
        sqlx::query!(
            "INSERT INTO nym_nodes
                (node_id, ed25519_identity_pubkey,
                    total_stake,
                    ip_addresses, mix_port,
                    x25519_sphinx_pubkey, node_role,
                    supported_roles, entry,
                    self_described,
                    bond_info,
                    active,
                    performance, last_updated_utc
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(node_id) DO UPDATE SET
                ed25519_identity_pubkey=excluded.ed25519_identity_pubkey,
                ip_addresses=excluded.ip_addresses,
                mix_port=excluded.mix_port,
                x25519_sphinx_pubkey=excluded.x25519_sphinx_pubkey,
                node_role=excluded.node_role,
                supported_roles=excluded.supported_roles,
                entry=excluded.entry,
                self_described=excluded.self_described,
                bond_info=excluded.bond_info,
                active=excluded.active,
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
            record.self_described,
            record.bond_info,
            record.active,
            record.performance,
            record.last_updated_utc,
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("node_id={}", record.node_id))?;
    }

    tx.commit().await?;

    Ok(inserted)
}

pub(crate) async fn get_active_node_bond_info(
    pool: &DbPool,
) -> anyhow::Result<HashMap<NodeId, NymNodeDetails>> {
    let mut conn = pool.acquire().await?;

    sqlx::query!(
        r#"SELECT
            node_id,
            bond_info as "bond_info: serde_json::Value"
        FROM
            nym_nodes
        WHERE
            bond_info IS NOT NULL
        AND
            active = true
        "#,
    )
    .fetch_all(&mut *conn)
    .await
    .map(|records| {
        records
            .into_iter()
            .filter_map(|record| {
                record
                    .bond_info
                    // only return details for nodes which have details stored
                    .and_then(|bond_info| serde_json::from_value::<NymNodeDetails>(bond_info).ok())
                    .map(|res| (record.node_id as NodeId, res))
            })
            .collect::<HashMap<_, _>>()
    })
    .map_err(From::from)
}

pub(crate) async fn get_active_node_descriptions(
    pool: &DbPool,
) -> anyhow::Result<HashMap<NodeId, NymNodeDescription>> {
    let mut conn = pool.acquire().await?;

    sqlx::query!(
        r#"SELECT
            node_id,
            self_described as "self_described: serde_json::Value"
        FROM
            nym_nodes
        WHERE
            self_described IS NOT NULL
        AND
            active = true
        "#,
    )
    .fetch_all(&mut *conn)
    .await
    .map(|records| {
        records
            .into_iter()
            .filter_map(|record| {
                record
                    .self_described
                    // only return details for nodes which have details stored
                    .and_then(|description| {
                        serde_json::from_value::<NymNodeDescription>(description).ok()
                    })
                    .map(|res| (record.node_id as NodeId, res))
            })
            .collect::<HashMap<_, _>>()
    })
    .map_err(From::from)
}
