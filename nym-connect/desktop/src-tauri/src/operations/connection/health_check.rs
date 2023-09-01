use crate::operations::directory::WELLKNOWN_DIR;
use nym_config::defaults::var_names::NETWORK_NAME;
use serde::{Deserialize, Serialize};

static HEALTH_CHECK_URL: &str = "connect/healthcheck.json";

#[derive(Serialize, Deserialize, Debug)]
struct ConnectionSuccess {
    status: String,
}

pub async fn run_health_check() -> bool {
    log::info!("Running network health check");
    let network_name = std::env::var(NETWORK_NAME).expect("network name not set");
    let url = format!("{}/{}/{}", WELLKNOWN_DIR, network_name, HEALTH_CHECK_URL);
    match crate::operations::http::socks5_get::<_, ConnectionSuccess>(url).await {
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
