use crate::error::{BackendError, Result};
use serde::de::DeserializeOwned;
use tap::TapFallible;

pub async fn socks5_get<U, T>(url: U) -> Result<T>
where
    U: reqwest::IntoUrl + std::fmt::Display,
    T: DeserializeOwned,
{
    log::info!(">>> GET {url}");
    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:1080")?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    let resp = client.get(url).send().await.tap_err(|err| {
        log::error!("<<< Request send error: {err}");
    })?;

    if resp.status().is_client_error() || resp.status().is_server_error() {
        log::error!("<<< {}", resp.status());
        return Err(BackendError::RequestFail {
            url: resp.url().clone(),
            status_code: resp.status(),
        });
    }

    let response_body = resp.text().await.tap_err(|err| {
        log::error!("<<< Request error: {err}");
    })?;
    log::info!("<<< {response_body}");

    Ok(serde_json::from_str(&response_body).tap_err(|err| {
        log::error!("<<< JSON parsing error: {err}");
    })?)
}
