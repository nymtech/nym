use crate::db::models::GatewayIdentityDto;
use crate::db::DbPool;
use futures_util::TryStreamExt;
use std::time::Duration;
use tracing::instrument;

pub(crate) mod models;
mod queue;
pub(crate) use queue::now_utc;

pub(crate) async fn spawn(pool: DbPool, refresh_interval: Duration) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = refresh_stale_testruns(&pool, refresh_interval).await {
                tracing::error!("{e}");
            }

            if let Err(e) = run(&pool).await {
                tracing::error!("Assigning testruns failed: {}", e);
            }
            tracing::debug!("Sleeping for {}s...", refresh_interval.as_secs());
            tokio::time::sleep(refresh_interval).await;
        }
    });
}

// TODO dz make number of max agents configurable

#[instrument(level = "debug", name = "testrun_queue", skip_all)]
async fn run(pool: &DbPool) -> anyhow::Result<()> {
    tracing::info!("Spawning testruns...");
    if pool.is_closed() {
        tracing::debug!("DB pool closed, returning early");
        return Ok(());
    }

    let mut conn = pool.acquire().await?;

    let gateways = sqlx::query_as!(
        GatewayIdentityDto,
        r#"SELECT
            gateway_identity_key as "gateway_identity_key!",
            bonded as "bonded: bool"
         FROM gateways
         ORDER BY last_testrun_utc"#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    // TODO dz this filtering could be done in SQL
    let gateways: Vec<GatewayIdentityDto> = gateways.into_iter().filter(|g| g.bonded).collect();

    tracing::debug!("Trying to queue {} testruns", gateways.len());
    let mut testruns_created = 0;
    for gateway in gateways {
        if let Err(e) = queue::try_queue_testrun(
            &mut conn,
            gateway.gateway_identity_key.clone(),
            // TODO dz read from config
            "127.0.0.1".to_string(),
        )
        .await
        // TODO dz measure how many were actually inserted and how many were skipped
        {
            tracing::debug!(
                "Skipping test for identity {} with error {}",
                &gateway.gateway_identity_key,
                e
            );
        } else {
            testruns_created += 1;
        }
    }
    tracing::debug!("{} testruns queued in total", testruns_created);

    Ok(())
}

#[instrument(level = "debug", skip_all)]
async fn refresh_stale_testruns(pool: &DbPool, refresh_interval: Duration) -> anyhow::Result<()> {
    let chrono_duration = chrono::Duration::from_std(refresh_interval)?;
    crate::db::queries::testruns::update_testruns_older_than(pool, chrono_duration).await?;

    Ok(())
}
