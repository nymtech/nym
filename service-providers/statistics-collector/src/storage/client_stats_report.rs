// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_statistics_common::report::ClientStatsReport;

pub(crate) type Result<T> = std::result::Result<T, sqlx::Error>;

#[derive(Clone)]
pub(crate) struct ClientStatsReportManager {
    connection_pool: sqlx::SqlitePool,
}

impl ClientStatsReportManager {
    /// Creates new instance of the `ClientStatsReportManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        ClientStatsReportManager { connection_pool }
    }

    pub async fn store_report(&mut self, report: ClientStatsReport) -> Result<()> {
        self.store_base(&report).await?;
        self.store_connection_stats(&report).await?;
        Ok(())
    }

    async fn store_base(&self, report: &ClientStatsReport) -> Result<()> {
        let report_day = report.last_update_time.date();
        sqlx::query!(
            "INSERT OR IGNORE INTO report (day, client_id, client_type, os_type, os_version, architecture) VALUES (?, ?, ?, ?, ?, ?)",
            report_day,
            report.client_id,
            report.client_type,
            report.os_information.os_type,
            report.os_information.os_version,
            report.os_information.os_arch
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn store_connection_stats(&self, report: &ClientStatsReport) -> Result<()> {
        sqlx::query!(
            "INSERT OR IGNORE INTO connection_stats (
                received_at, 
                client_id, 
                mixnet_entry_spent, 
                vpn_entry_spent, 
                mixnet_exit_spent, 
                vpn_exit_spent, 
                wg_exit_country_code, 
                mix_exit_country_code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            report.last_update_time,
            report.client_id,
            report.connection_stats.mixnet_entry_spent,
            report.connection_stats.vpn_entry_spent,
            report.connection_stats.mixnet_exit_spent,
            report.connection_stats.vpn_exit_spent,
            report.connection_stats.wg_exit_country_code,
            report.connection_stats.mix_exit_country_code
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
