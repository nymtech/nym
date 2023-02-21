use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum ApiClientError {
    #[error("{source}")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
    #[error("{source}")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
    },
    #[error("{0}")]
    Status(String),
}

const API_BASE_URL: &str = "https://growth-api.nymtech.net";

// For development mode, switch to this
// const API_BASE_URL: &str = "http://localhost:8000";

#[derive(Debug, Clone)]
pub struct GrowthApiClient {
    base_url: String,
}

impl GrowthApiClient {
    pub fn new(resource_base: &str) -> Self {
        let base_url = std::env::var("API_BASE_URL").unwrap_or_else(|_| API_BASE_URL.to_string());
        GrowthApiClient {
            base_url: format!("{}{}", base_url, resource_base),
        }
    }

    pub fn registrations() -> Registrations {
        Registrations::new(GrowthApiClient::new("/v1/tne"))
    }

    pub fn daily_draws() -> DailyDraws {
        DailyDraws::new(GrowthApiClient::new("/v1/tne/daily_draw"))
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiClientError> {
        log::info!(">>> GET {}", url);
        let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:1080")?;
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        match client.get(format!("{}{}", self.base_url, url)).send().await {
            Ok(res) => {
                if res.status().is_client_error() || res.status().is_server_error() {
                    log::error!("<<< {}", res.status());
                    return Err(ApiClientError::Status(res.status().to_string()));
                }
                match res.text().await {
                    Ok(response_body) => {
                        log::info!("<<< {}", response_body);
                        match serde_json::from_str(&response_body) {
                            Ok(res) => Ok(res),
                            Err(e) => {
                                log::error!("<<< JSON parsing error: {}", e);
                                Err(e.into())
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("<<< Request error: {}", e);
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                log::error!("<<< Response parsing error: {}", e);
                Err(e.into())
            }
        }
    }

    pub(crate) async fn post<REQ: Serialize + ?Sized, RESP: DeserializeOwned>(
        &self,
        url: &str,
        body: &REQ,
    ) -> Result<RESP, ApiClientError> {
        log::info!(">>> POST {}", url);
        let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:1080")?;
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        match client
            .post(format!("{}{}", self.base_url, url))
            .json(body)
            .send()
            .await
        {
            Ok(res) => {
                if res.status().is_client_error() || res.status().is_server_error() {
                    log::error!("<<< {}", res.status());
                    return Err(ApiClientError::Status(res.status().to_string()));
                }
                match res.text().await {
                    Ok(response_body) => {
                        log::info!("<<< {}", response_body);
                        match serde_json::from_str(&response_body) {
                            Ok(res) => Ok(res),
                            Err(e) => {
                                log::error!("<<< JSON parsing error: {}", e);
                                Err(e.into())
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("<<< Request error: {}", e);
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                log::error!("<<< Response parsing error: {}", e);
                Err(e.into())
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientIdPartial {
    pub client_id: String,
    pub client_id_signature: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Registration {
    pub id: String,
    pub client_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ping {
    pub client_id: String,
    pub timestamp: String,
}

pub struct Registrations {
    client: GrowthApiClient,
}

impl Registrations {
    pub fn new(client: GrowthApiClient) -> Self {
        Registrations { client }
    }

    pub async fn register(
        &self,
        registration: &ClientIdPartial,
    ) -> Result<Registration, ApiClientError> {
        self.client.post("/register", &registration).await
    }

    #[allow(dead_code)]
    pub async fn unregister(&self, registration: &ClientIdPartial) -> Result<(), ApiClientError> {
        self.client.post("/unregister", &registration).await
    }

    #[allow(dead_code)]
    pub async fn status(&self, registration: &ClientIdPartial) -> Result<(), ApiClientError> {
        self.client.post("/status", &registration).await
    }

    pub async fn ping(&self, registration: &ClientIdPartial) -> Result<(), ApiClientError> {
        self.client.post("/ping", &registration).await
    }

    #[allow(dead_code)]
    pub async fn health(&self) -> Result<(), ApiClientError> {
        self.client.get("/health").await
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DrawEntryPartial {
    pub draw_id: String,
    pub client_id: String,
    pub client_id_signature: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DrawEntry {
    pub id: String,
    pub draw_id: String,
    pub timestamp: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DrawWithWordOfTheDay {
    pub id: String,
    pub start_utc: String,
    pub end_utc: String,
    pub word_of_the_day: Option<String>,
    pub last_modified: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaimPartial {
    pub draw_id: String,
    pub registration_id: String,
    pub client_id: String,
    pub client_id_signature: String,
    pub wallet_address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Winner {
    pub id: String,
    pub client_id: String,
    pub draw_id: String,
    pub timestamp: String,
    pub winner_reg_id: String,
    pub winner_wallet_address: Option<String>,
    pub winner_claim_timestamp: Option<String>,
}

pub struct DailyDraws {
    client: GrowthApiClient,
}

impl DailyDraws {
    pub fn new(client: GrowthApiClient) -> Self {
        DailyDraws { client }
    }

    pub async fn current(&self) -> Result<DrawWithWordOfTheDay, ApiClientError> {
        self.client.get("/current").await
    }

    pub async fn next(&self) -> Result<DrawWithWordOfTheDay, ApiClientError> {
        self.client.get("/next").await
    }

    #[allow(dead_code)]
    pub async fn status(&self, draw_id: &str) -> Result<DrawWithWordOfTheDay, ApiClientError> {
        self.client
            .get(format!("/status/{}", draw_id).as_str())
            .await
    }

    pub async fn enter(&self, entry: &DrawEntryPartial) -> Result<DrawEntry, ApiClientError> {
        self.client.post("/enter", entry).await
    }

    pub async fn entries(
        &self,
        client_id: &ClientIdPartial,
    ) -> Result<Vec<DrawEntry>, ApiClientError> {
        self.client.post("/entries", client_id).await
    }

    pub async fn claim(&self, claim: &ClaimPartial) -> Result<Winner, ApiClientError> {
        self.client.post("/claim", claim).await
    }
}
