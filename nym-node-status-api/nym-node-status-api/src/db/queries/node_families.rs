// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::Storage;
use crate::db::models::{NodeFamilyDto, NodeFamilyInsertRecord, NodeFamilyMemberDto};
use futures_util::TryStreamExt;
use tracing::instrument;

impl Storage {
    /// Replace the node-families snapshot atomically. Wipes both
    /// `node_families` and `node_family_members` (cascade) and re-inserts
    /// the provided records inside a single transaction so reads never
    /// observe a partial state.
    #[instrument(level = "debug", skip_all, fields(family_records = family_records.len()))]
    pub(crate) async fn update_node_families(
        &self,
        family_records: Vec<NodeFamilyInsertRecord>,
    ) -> anyhow::Result<usize> {
        let mut tx = self.pool.begin().await?;

        // ON DELETE CASCADE on the members table wipes both sides
        sqlx::query!("DELETE FROM node_families")
            .execute(&mut *tx)
            .await?;

        let inserted = family_records.len();
        for record in family_records {
            sqlx::query!(
                "INSERT INTO node_families
                    (family_id, name, description, owner, family_stake_unym, members_count, created_at, last_updated_utc)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                record.family_id,
                record.name,
                record.description,
                record.owner,
                record.family_stake_unym,
                record.members_count,
                record.created_at,
                record.last_updated_utc,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to INSERT family_id={}: {}", record.family_id, e)
            })?;

            for member in record.members {
                sqlx::query!(
                    "INSERT INTO node_family_members (node_id, family_id, joined_at)
                     VALUES ($1, $2, $3)",
                    member.node_id,
                    record.family_id,
                    member.joined_at,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to INSERT family_id={} member node_id={}: {}",
                        record.family_id,
                        member.node_id,
                        e
                    )
                })?;
            }
        }

        tx.commit().await?;
        Ok(inserted)
    }

    /// Read every cached family.
    pub(crate) async fn get_all_node_families(&self) -> anyhow::Result<Vec<NodeFamilyDto>> {
        sqlx::query_as!(
            NodeFamilyDto,
            r#"SELECT
                family_id,
                name,
                description,
                owner,
                family_stake_unym,
                members_count,
                created_at
             FROM node_families
             ORDER BY family_id"#,
        )
        .fetch(&self.pool)
        .try_collect::<Vec<_>>()
        .await
        .map_err(From::from)
    }

    /// Read every cached `(node_id, family_id)` membership pair.
    pub(crate) async fn get_all_node_family_members(
        &self,
    ) -> anyhow::Result<Vec<NodeFamilyMemberDto>> {
        sqlx::query_as!(
            NodeFamilyMemberDto,
            r#"SELECT node_id, family_id
             FROM node_family_members"#,
        )
        .fetch(&self.pool)
        .try_collect::<Vec<_>>()
        .await
        .map_err(From::from)
    }
}
