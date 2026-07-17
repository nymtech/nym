// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::FreeTierRecord;
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct FreeTierManager {
    connection_pool: sqlx::SqlitePool,
}

impl FreeTierManager {
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        FreeTierManager { connection_pool }
    }

    /// Create or replace the free-tier record for a public key.
    pub(crate) async fn set_record(
        &self,
        public_key: &str,
        granted_at: OffsetDateTime,
        is_free: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT OR REPLACE INTO free_tier_state (public_key, granted_at, is_free)
                VALUES (?, ?, ?)
            "#,
            public_key,
            granted_at,
            is_free,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Retrieve the free-tier record for a public key, if any.
    pub(crate) async fn get_record(
        &self,
        public_key: &str,
    ) -> Result<Option<FreeTierRecord>, sqlx::Error> {
        sqlx::query_as!(
            FreeTierRecord,
            r#"
                SELECT
                    public_key as "public_key!",
                    granted_at as "granted_at!: OffsetDateTime",
                    is_free as "is_free!: bool"
                FROM free_tier_state
                WHERE public_key = ?
                LIMIT 1
            "#,
            public_key,
        )
        .fetch_optional(&self.connection_pool)
        .await
    }

    /// Remove the free-tier record for a public key.
    pub(crate) async fn remove_record(&self, public_key: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                DELETE FROM free_tier_state
                WHERE public_key = ?
            "#,
            public_key,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Update only the `is_free` flag for a public key (e.g. on upgrade to paid),
    /// leaving `granted_at` intact so the claim guard still applies. No-op if the
    /// record does not exist.
    pub(crate) async fn set_is_free(
        &self,
        public_key: &str,
        is_free: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE free_tier_state
                SET is_free = ?
                WHERE public_key = ?
            "#,
            is_free,
            public_key,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_manager() -> FreeTierManager {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        FreeTierManager::new(pool)
    }

    #[tokio::test]
    async fn free_tier_record_round_trip() {
        let mgr = test_manager().await;

        // absent by default
        assert!(mgr.get_record("pk1").await.unwrap().is_none());

        // create
        let t1 = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        mgr.set_record("pk1", t1, true).await.unwrap();
        let rec = mgr.get_record("pk1").await.unwrap().unwrap();
        assert_eq!(rec.public_key, "pk1");
        assert_eq!(rec.granted_at.unix_timestamp(), t1.unix_timestamp());
        assert!(rec.is_free);

        // replace (fresh claim, or upgrade flipping is_free)
        let t2 = OffsetDateTime::from_unix_timestamp(1_700_100_000).unwrap();
        mgr.set_record("pk1", t2, false).await.unwrap();
        let rec = mgr.get_record("pk1").await.unwrap().unwrap();
        assert_eq!(rec.granted_at.unix_timestamp(), t2.unix_timestamp());
        assert!(!rec.is_free);

        // narrow is_free update leaves granted_at intact (the upgrade-to-paid path)
        mgr.set_record("pk1", t2, true).await.unwrap();
        mgr.set_is_free("pk1", false).await.unwrap();
        let rec = mgr.get_record("pk1").await.unwrap().unwrap();
        assert!(!rec.is_free);
        assert_eq!(rec.granted_at.unix_timestamp(), t2.unix_timestamp());

        // a different key is independent
        mgr.set_record("pk2", t1, true).await.unwrap();
        assert!(mgr.get_record("pk2").await.unwrap().unwrap().is_free);

        // remove
        mgr.remove_record("pk1").await.unwrap();
        assert!(mgr.get_record("pk1").await.unwrap().is_none());
        assert!(mgr.get_record("pk2").await.unwrap().is_some());
    }
}
