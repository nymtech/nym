// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::models::Payment;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    pub(crate) async fn get_payment(&self, serial_number: &str) -> Result<Payment, sqlx::Error> {
        sqlx::query_as!(
            Payment,
            r#"
                SELECT *
                FROM payments 
                WHERE serial_number = ?
            "#,
            serial_number
        )
        .fetch_one(&self.connection_pool)
        .await
    }

    pub(crate) async fn update_payment(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                UPDATE payments
                SET paid = true
                WHERE id = ?
            "#,
            id
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }
}
