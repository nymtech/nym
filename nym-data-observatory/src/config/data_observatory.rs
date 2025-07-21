use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DataObservatoryConfig {
    pub webhooks: Vec<WebhookConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub description: Option<String>,
    pub webhook_url: String,
    pub watch_for_chain_message_types: Vec<String>,
    pub authentication: Option<HttpAuthenticationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpAuthenticationOptions {
    AuthorizationBearerToken { token: String },
}
