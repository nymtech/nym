use anyhow::{anyhow, Result};
use models::{ConnectionInfoDto, DailyActiveDeviceDto};
use sqlx::{migrate::Migrator, postgres::PgConnectOptions};
use std::str::FromStr;

pub(crate) mod models;

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
        active_device: DailyActiveDeviceDto,
        connection_info: Option<ConnectionInfoDto>,
    ) -> Result<()> {
        self.store_device(active_device).await?;
        if let Some(connection_info) = connection_info {
            self.store_connection_stats(connection_info).await?;
        }
        Ok(())
    }

    // Interestingly enough, because gateway-storage is using the `chrono` feature of sqlx and in 0.7.4 it takes priority over the `time` one, we cannot use the query! macro here.
    // Due to features unification, the binary will not compile when built from the workspace root because it will expect `chrono` types.
    // As a consequence, there is no compile time verification of these queries.
    async fn store_device(&self, active_device: DailyActiveDeviceDto) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO active_device (
                day,
                device_id,
                os_type,
                os_version,
                architecture,
                app_version,
                user_agent,
                from_mixnet)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (device_id, day) DO NOTHING"#,
            active_device.day as time::Date,
            active_device.stats_id,
            active_device.os_type,
            active_device.os_version,
            active_device.os_arch,
            active_device.app_version,
            active_device.user_agent,
            active_device.from_mixnet
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    async fn store_connection_stats(&self, connection_info: ConnectionInfoDto) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO connection_stats (
                received_at,
                connection_time_ms,
                two_hop,
                source_ip,
                country_code,
                from_mixnet) VALUES ($1::timestamptz, $2, $3, $4, $5, $6)"#,
            connection_info.received_at as time::OffsetDateTime,
            connection_info.connection_time_ms,
            connection_info.two_hop,
            connection_info.received_from,
            connection_info.country_code,
            connection_info.from_mixnet
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
