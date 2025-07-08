use futures_util::TryStreamExt;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_validator_client::{
    client::{NodeId, NymNodeDetails},
    models::NymNodeDescription,
};
use sqlx::Row;
use std::collections::HashMap;
use tracing::instrument;

use crate::{
    db::{
        models::{NymNodeDto, NymNodeInsertRecord},
        DbConnection, DbPool,
    },
    node_scraper::helpers::NodeDescriptionResponse,
};

pub(crate) async fn get_all_nym_nodes(pool: &DbPool) -> anyhow::Result<Vec<NymNodeDto>> {
    let mut conn = pool.acquire().await?;

    crate::db::query_as::<NymNodeDto>(
        r#"SELECT
            node_id,
            ed25519_identity_pubkey,
            total_stake,
            ip_addresses,
            mix_port,
            x25519_sphinx_pubkey,
            node_role,
            supported_roles,
            entry,
            performance,
            self_described,
            bond_info
        FROM
            nym_nodes
        ORDER BY
            node_id
        "#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<NymNodeDto>>()
    .await
    .map_err(From::from)
}

/// if a node doesn't expose its self-described endpoint, it can't route traffic
/// - https://nym.com/docs/operators/nodes/nym-node/bonding
///
/// same if it's not bonded in the mixnet smart contract
/// - https://nym.com/docs/operators/tokenomics/mixnet-rewards#rewarded-set-selection
pub(crate) async fn get_described_bonded_nym_nodes(
    pool: &DbPool,
) -> anyhow::Result<Vec<NymNodeDto>> {
    let mut conn = pool.acquire().await?;

    crate::db::query_as::<NymNodeDto>(
        r#"SELECT
            node_id,
            ed25519_identity_pubkey,
            total_stake,
            ip_addresses,
            mix_port,
            x25519_sphinx_pubkey,
            node_role,
            supported_roles,
            entry,
            performance,
            self_described,
            bond_info
        FROM
            nym_nodes
        WHERE
            self_described IS NOT NULL
        AND
            bond_info IS NOT NULL
        "#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<NymNodeDto>>()
    .await
    .map_err(From::from)
}

#[instrument(level = "debug", skip_all, fields(node_records=node_records.len()))]
pub(crate) async fn update_nym_nodes(
    pool: &DbPool,
    node_records: Vec<NymNodeInsertRecord>,
) -> anyhow::Result<usize> {
    let mut tx = pool.begin().await?;

    crate::db::query(
        "UPDATE nym_nodes
        SET
            self_described = NULL,
            bond_info = NULL",
    )
    .execute(&mut *tx)
    .await?;

    let inserted = node_records.len();
    for record in node_records {
        // https://www.sqlite.org/lang_upsert.html
        crate::db::query(
            "INSERT INTO nym_nodes
                (node_id, ed25519_identity_pubkey,
                    total_stake,
                    ip_addresses, mix_port,
                    x25519_sphinx_pubkey, node_role,
                    supported_roles, entry,
                    self_described,
                    bond_info,
                    performance, last_updated_utc
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                performance=excluded.performance,
                last_updated_utc=excluded.last_updated_utc
                ;",
        )
        .bind(record.node_id)
        .bind(record.ed25519_identity_pubkey)
        .bind(record.total_stake)
        .bind(record.ip_addresses)
        .bind(record.mix_port)
        .bind(record.x25519_sphinx_pubkey)
        .bind(record.node_role)
        .bind(record.supported_roles)
        .bind(record.entry)
        .bind(record.self_described)
        .bind(record.bond_info)
        .bind(record.performance)
        .bind(record.last_updated_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to INSERT node_id={}: {}", record.node_id, e))?;
    }

    tx.commit().await?;

    Ok(inserted)
}

pub(crate) async fn get_described_node_bond_info(
    pool: &DbPool,
) -> anyhow::Result<HashMap<NodeId, NymNodeDetails>> {
    let mut conn = pool.acquire().await?;

    crate::db::query(
        r#"SELECT
            node_id,
            bond_info
        FROM
            nym_nodes
        WHERE
            bond_info IS NOT NULL
        AND
            self_described IS NOT NULL
        "#,
    )
    .fetch_all(&mut *conn)
    .await
    .map(|records| {
        records
            .into_iter()
            .filter_map(|record| {
                let node_id: i32 = record.try_get("node_id").ok()?;
                let bond_info: serde_json::Value = record.try_get("bond_info").ok()?;
                serde_json::from_value::<NymNodeDetails>(bond_info)
                    .ok()
                    .map(|res| (node_id as i64 as NodeId, res))
            })
            .collect::<HashMap<_, _>>()
    })
    .map_err(From::from)
}

pub(crate) async fn get_node_self_description(
    pool: &DbPool,
) -> anyhow::Result<HashMap<NodeId, NymNodeDescription>> {
    let mut conn = pool.acquire().await?;

    crate::db::query(
        r#"SELECT
            node_id,
            self_described
        FROM
            nym_nodes
        WHERE
            self_described IS NOT NULL
        ORDER BY
            node_id
        "#,
    )
    .fetch_all(&mut *conn)
    .await
    .map(|records| {
        records
            .into_iter()
            .filter_map(|record| {
                let node_id: i32 = record.try_get("node_id").ok()?;
                let self_described: serde_json::Value = record.try_get("self_described").ok()?;
                serde_json::from_value::<NymNodeDescription>(self_described)
                    .ok()
                    .map(|res| (node_id as i64 as NodeId, res))
            })
            .collect::<HashMap<_, _>>()
    })
    .map_err(From::from)
}

pub(crate) async fn get_bonded_node_description(
    pool: &DbPool,
) -> anyhow::Result<HashMap<NodeId, NodeDescription>> {
    let mut conn = pool.acquire().await?;

    crate::db::query(
        r#"SELECT
            nd.node_id,
            moniker,
            website,
            security_contact,
            details
        FROM
            nym_node_descriptions nd
        INNER JOIN
            nym_nodes
        WHERE
            bond_info IS NOT NULL
        "#,
    )
    .fetch_all(&mut *conn)
    .await
    .map(|records| {
        records
            .into_iter()
            .map(|elem| {
                let node_id: i64 = elem.try_get("node_id").unwrap_or(0);
                let node_id: NodeId = node_id.try_into().unwrap_or_default();
                (
                    node_id,
                    NodeDescription {
                        moniker: elem.try_get("moniker").unwrap_or_default(),
                        website: elem.try_get("website").unwrap_or_default(),
                        security_contact: elem.try_get("security_contact").unwrap_or_default(),
                        details: elem.try_get("details").unwrap_or_default(),
                    },
                )
            })
            .collect::<HashMap<NodeId, NodeDescription>>()
    })
    .map_err(From::from)
}

pub(crate) async fn insert_nym_node_description(
    conn: &mut DbConnection,
    node_id: i64,
    description: NodeDescriptionResponse,
    timestamp: i64,
) -> anyhow::Result<()> {
    crate::db::query(
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
    )
    .bind(node_id)
    .bind(description.moniker)
    .bind(description.website)
    .bind(description.security_contact)
    .bind(description.details)
    .bind(timestamp)
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}
