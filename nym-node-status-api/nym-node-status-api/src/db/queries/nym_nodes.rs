use crate::db::models::NymNodeDescriptionDeHelper;
use futures_util::TryStreamExt;
use nym_node_requests::api::v1::node::models::NodeDescription;
use nym_validator_client::{
    client::{NodeId, NymNodeDetails},
    models::NymNodeDescription,
};
use std::collections::HashMap;
use tracing::{instrument, warn};

use crate::db::DbConnection;
use crate::http::models::DailyStats;
use crate::{
    db::{
        models::{NymNodeDto, NymNodeInsertRecord},
        DbPool,
    },
    node_scraper::helpers::NodeDescriptionResponse,
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
            bond_info as "bond_info: serde_json::Value"
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
            bond_info as "bond_info: serde_json::Value"
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

    sqlx::query!(
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
        sqlx::query!(
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
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
            record.performance,
            record.last_updated_utc as i32,
        )
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

    sqlx::query!(
        r#"SELECT
            node_id,
            bond_info as "bond_info: serde_json::Value"
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

pub(crate) async fn get_node_self_description(
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
                let node_id = record.node_id;
                record
                    .self_described
                    // only return details for nodes which have details stored
                    .and_then(|description| {
                        serde_json::from_value::<NymNodeDescriptionDeHelper>(description)
                            .inspect_err(|err| {
                                warn!("malformed description data for node {node_id}: {err}")
                            })
                            .ok()
                    })
                    .map(|res| (record.node_id as NodeId, res.into()))
            })
            .collect::<HashMap<_, _>>()
    })
    .map_err(From::from)
}

pub(crate) async fn get_bonded_node_description(
    pool: &DbPool,
) -> anyhow::Result<HashMap<NodeId, NodeDescription>> {
    let mut conn = pool.acquire().await?;

    sqlx::query!(
        r#"SELECT
            nd.node_id,
            moniker,
            website,
            security_contact,
            details
        FROM
            nym_node_descriptions nd
        INNER JOIN
            nym_nodes nn on nd.node_id = nn.node_id
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
                let node_id: NodeId = elem.node_id.try_into().unwrap_or_default();
                (
                    node_id,
                    NodeDescription {
                        moniker: elem.moniker.unwrap_or_default(),
                        website: elem.website.unwrap_or_default(),
                        security_contact: elem.security_contact.unwrap_or_default(),
                        details: elem.details.unwrap_or_default(),
                    },
                )
            })
            .collect::<HashMap<NodeId, NodeDescription>>()
    })
    .map_err(From::from)
}

pub(crate) async fn insert_nym_node_description(
    conn: &mut DbConnection,
    node_id: &i64,
    description: &NodeDescriptionResponse,
    timestamp: i64,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO nym_node_descriptions (
            node_id, moniker, website, security_contact, details, last_updated_utc
        ) VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (node_id) DO UPDATE SET
            moniker = excluded.moniker,
            website = excluded.website,
            security_contact = excluded.security_contact,
            details = excluded.details,
            last_updated_utc = excluded.last_updated_utc
        "#,
        *node_id as i32,
        description.moniker,
        description.website,
        description.security_contact,
        description.details,
        timestamp as i32,
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn get_daily_stats(pool: &DbPool) -> anyhow::Result<Vec<DailyStats>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        DailyStats,
        r#"
        SELECT
            date_utc as "date_utc!",
            SUM(total_stake)::bigint as "total_stake!: i64",
            SUM(packets_received)::bigint as "total_packets_received!: i64",
            SUM(packets_sent)::bigint as "total_packets_sent!: i64",
            SUM(packets_dropped)::bigint as "total_packets_dropped!: i64"
        FROM nym_node_daily_mixing_stats
        GROUP BY date_utc
        ORDER BY date_utc ASC
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(items)
}
