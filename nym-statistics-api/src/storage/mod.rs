use anyhow::{anyhow, Result};
use nym_statistics_common::report::vpn_client::{
    StaticInformationReport, UsageReport, VpnClientStatsReport,
};
use sqlx::{migrate::Migrator, postgres::PgConnectOptions};
use std::{net::SocketAddr, str::FromStr};
use time::{Date, OffsetDateTime};

pub(crate) type DbPool = sqlx::PgPool;
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub(crate) struct StatisticsStorage {
    connection_pool: DbPool,
}

impl StatisticsStorage {
    pub async fn init(
        connection_url: String,
        user: String,
        password: String,
        port: u16,
    ) -> Result<Self> {
        let connect_options = PgConnectOptions::from_str(&connection_url)?
            .port(port)
            .username(&user)
            .password(&password)
            .application_name(nym_bin_common::bin_info!().binary_name);

        let pool = sqlx::PgPool::connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR.run(&pool).await?;

        Ok(StatisticsStorage {
            connection_pool: pool,
        })
    }

    pub(crate) async fn store_vpn_client_report(
        &mut self,
        report: VpnClientStatsReport,
        datetime: OffsetDateTime,
        origin: SocketAddr,
    ) -> Result<()> {
        self.store_device(
            report.stats_id.clone(),
            &report.static_information,
            datetime.date(),
        )
        .await?;
        if let Some(usage_report) = report.basic_usage {
            self.store_connection_stats(report.stats_id.clone(), origin, datetime, &usage_report)
                .await?;
        }
        Ok(())
    }

    // Interestingly enough, because gateway-storage is using the `chrono` feature of sqlx and in 0.7.4 it takes priority over the `time` one, we cannot use the query! macro here.
    // Due to features unification, the binary will not compile when built from the workspace root because it will expect `chrono` types.
    // As a consequence, there is no compile time verification of these queries.
    async fn store_device(
        &self,
        stats_id: String,
        report: &StaticInformationReport,
        day: Date,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO active_device (
                day, 
                device_id, 
                os_type, 
                os_version, 
                architecture, 
                app_version) 
                VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (device_id, day) DO NOTHING",
        )
        .bind(day)
        .bind(stats_id)
        .bind(report.os_type.clone())
        .bind(report.os_version.clone())
        .bind(report.os_arch.clone())
        .bind(report.app_version.clone())
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    // We're cannot use the query! macro because of the above comment
    async fn store_connection_stats(
        &self,
        stats_id: String,
        received_from: SocketAddr,
        received_at: OffsetDateTime,
        report: &UsageReport,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO connection_stats (
                received_at, 
                device_id, 
                connection_time_ms, 
                two_hop, 
                gateway_ip) VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (device_id, received_at) DO NOTHING",
        )
        .bind(received_at)
        .bind(stats_id)
        .bind(report.connection_time_ms)
        .bind(report.two_hop)
        .bind(received_from.to_string())
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
