use anyhow::{Result, anyhow};
use sqlx::{
    Executor,
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{path::PathBuf, str::FromStr};

use crate::storage::models::{StatsReportV1Dto, StatsReportV2Dto};

pub(crate) mod models;

pub(crate) type DbPool = sqlx::PgPool;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
const SET_SEARCH_PATH: &str = "SET search_path = private_statistics_api";

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
        ssl_cert_path: Option<PathBuf>,
    ) -> Result<Self> {
        let mut connect_options = PgConnectOptions::from_str(&connection_url)?
            .port(port)
            .username(&user)
            .password(&password)
            .application_name(nym_bin_common::bin_info!().binary_name);

        if let Some(ssl_cert) = ssl_cert_path {
            connect_options = connect_options
                .ssl_mode(sqlx::postgres::PgSslMode::Require)
                .ssl_root_cert(ssl_cert);
        }

        // This is a custom connection so the _sqlx_migrations table is not written in the public schema
        // It then ensures we'll only write in the given schema, allowing to have the schema name only once here
        // Ref : https://github.com/launchbadge/sqlx/issues/1835
        let pool = PgPoolOptions::new()
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    conn.execute(SET_SEARCH_PATH).await?;
                    Ok(())
                })
            })
            .connect_with(connect_options)
            .await
            .map_err(|err| anyhow!("Failed to connect to {}: {}", &connection_url, err))?;

        MIGRATOR.run(&pool).await?;

        Ok(StatisticsStorage {
            connection_pool: pool,
        })
    }

    pub(crate) async fn store_vpn_client_report(
        &mut self,
        report_v1: StatsReportV1Dto,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO report_v1 (
                received_at,
                source_ip,
                device_id,
                from_mixnet,
                os_type,
                os_version,
                architecture,
                app_version,
                user_agent,
                connection_time_ms,
                two_hop,
                country_code)
                VALUES ($1::timestamptz, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
            report_v1.received_at as time::OffsetDateTime,
            report_v1.received_from,
            report_v1.stats_id,
            report_v1.from_mixnet,
            report_v1.os_type,
            report_v1.os_version,
            report_v1.os_arch,
            report_v1.app_version,
            report_v1.user_agent,
            report_v1.connection_time_ms,
            report_v1.two_hop,
            report_v1.country_code
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn store_vpn_client_report_v2(
        &self,
        report_v2: StatsReportV2Dto,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO report_v2(
                received_at,
                source_ip,
                from_mixnet,
                country_code,
                device_id,
                os_type,
                os_version,
                architecture,
                app_version,
                user_agent,
                start_day,
                connection_time_ms,
                two_hop,
                session_duration_min,
                exit_id,
                exit_country_code,
                error)
                VALUES ($1::timestamptz, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)"#,
            report_v2.received_at as time::OffsetDateTime,
            report_v2.received_from,
            report_v2.from_mixnet,
            report_v2.country_code,
            report_v2.stats_id,
            report_v2.os_type,
            report_v2.os_version,
            report_v2.os_arch,
            report_v2.app_version,
            report_v2.user_agent,
            report_v2.start_day,
            report_v2.connection_time_ms,
            report_v2.two_hop,
            report_v2.session_duration_min,
            report_v2.exit_id,
            report_v2.exit_country_code,
            report_v2.error
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
