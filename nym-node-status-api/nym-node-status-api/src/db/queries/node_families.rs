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
    ///
    /// Both inserts are batched via `UNNEST(..)`, so the whole refresh is a
    /// constant number of round trips regardless of how many families/members
    /// the snapshot contains.
    #[instrument(level = "debug", skip_all, fields(family_records = family_records.len()))]
    pub(crate) async fn update_node_families(
        &self,
        family_records: Vec<NodeFamilyInsertRecord>,
    ) -> anyhow::Result<usize> {
        let inserted = family_records.len();

        // Reshape the row-major records into column-major vectors so we can
        // bind each column as a Postgres array and let `UNNEST` expand them
        // back into rows.
        let mut family_ids: Vec<i64> = Vec::with_capacity(inserted);
        let mut names: Vec<String> = Vec::with_capacity(inserted);
        let mut descriptions: Vec<String> = Vec::with_capacity(inserted);
        let mut owners: Vec<String> = Vec::with_capacity(inserted);
        let mut family_stakes: Vec<Option<i64>> = Vec::with_capacity(inserted);
        let mut members_counts: Vec<i32> = Vec::with_capacity(inserted);
        let mut created_ats: Vec<i64> = Vec::with_capacity(inserted);
        let mut last_updated_utcs: Vec<i64> = Vec::with_capacity(inserted);

        let total_members: usize = family_records.iter().map(|f| f.members.len()).sum();
        let mut member_node_ids: Vec<i64> = Vec::with_capacity(total_members);
        let mut member_family_ids: Vec<i64> = Vec::with_capacity(total_members);
        let mut member_joined_ats: Vec<i64> = Vec::with_capacity(total_members);

        for record in family_records {
            let family_id = record.family_id;
            family_ids.push(family_id);
            names.push(record.name);
            descriptions.push(record.description);
            owners.push(record.owner);
            family_stakes.push(record.family_stake_unym);
            members_counts.push(record.members_count);
            created_ats.push(record.created_at);
            last_updated_utcs.push(record.last_updated_utc);

            for member in record.members {
                member_node_ids.push(member.node_id);
                member_family_ids.push(family_id);
                member_joined_ats.push(member.joined_at);
            }
        }

        let mut tx = self.pool.begin().await?;

        // ON DELETE CASCADE on the members table wipes both sides
        sqlx::query!("DELETE FROM node_families")
            .execute(&mut *tx)
            .await?;

        sqlx::query!(
            "INSERT INTO node_families
                (family_id, name, description, owner, family_stake_unym, members_count, created_at, last_updated_utc)
             SELECT * FROM UNNEST(
                $1::BIGINT[], $2::TEXT[], $3::TEXT[], $4::TEXT[],
                $5::BIGINT[], $6::INTEGER[], $7::BIGINT[], $8::BIGINT[]
             )",
            &family_ids[..],
            &names[..],
            &descriptions[..],
            &owners[..],
            &family_stakes[..] as &[Option<i64>],
            &members_counts[..],
            &created_ats[..],
            &last_updated_utcs[..],
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO node_family_members (node_id, family_id, joined_at)
             SELECT * FROM UNNEST($1::BIGINT[], $2::BIGINT[], $3::BIGINT[])",
            &member_node_ids[..],
            &member_family_ids[..],
            &member_joined_ats[..],
        )
        .execute(&mut *tx)
        .await?;

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
