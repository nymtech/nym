use serde::{Deserialize, Serialize};

static HEALTH_CHECK_URL: &str = "https://nymtech.net/.wellknown/connect/healthcheck.json";

#[derive(Serialize, Deserialize, Debug)]
struct ConnectionSuccess {
    status: String,
}

pub async fn run_health_check() -> bool {
    log::info!("Running network health check");
    match crate::operations::http::socks5_get::<_, ConnectionSuccess>(HEALTH_CHECK_URL).await {
        Ok(res) if res.status == "ok" => {
            log::info!("✅✅✅ Healthcheck success!");
            true
        }
        Ok(res) => {
            log::error!("⛔⛔⛔ Healthcheck failed with status: {}", res.status);
            false
        }
        Err(err) => {
            log::error!("⛔⛔⛔ Healthcheck failed: {err}");
            false
        }
    }
}
