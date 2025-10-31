use crate::models::{TestrunAssignmentWithTickets, get_testrun, submit_results, submit_results_v2};
use anyhow::bail;
use api::ApiPaths;
use nym_crypto::asymmetric::ed25519::{PrivateKey, Signature};
use tracing::{instrument, warn};

mod api;
pub mod auth;
pub mod models;

pub struct NsApiClient {
    api: ApiPaths,
    client: reqwest::Client,
    auth_key: PrivateKey,
}

impl NsApiClient {
    pub fn new(server_ip: &str, server_port: u16, auth_key: PrivateKey) -> Self {
        let server_address = format!("{server_ip}:{server_port}");
        let api = ApiPaths::new(server_address);
        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .inspect_err(|err| {
                warn!(
                    "Failed to create client with user agent, falling back to default ({})",
                    err
                )
            })
            // failing to set user agent shouldn't be a critical error
            .unwrap_or_default();

        Self {
            api,
            client,
            auth_key,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn request_testrun(&self) -> anyhow::Result<Option<TestrunAssignmentWithTickets>> {
        let target_url = self.api.request_testrun();

        let payload = get_testrun::Payload {
            agent_public_key: self.auth_key.public_key(),
            timestamp: time::UtcDateTime::now().unix_timestamp(),
        };
        let signature = self.sign_message(&payload)?;
        let request = get_testrun::GetTestrunRequest { payload, signature };

        let res = self.client.get(target_url).json(&request).send().await?;
        let status = res.status();
        let response_text = res.text().await?;

        if status.is_client_error() {
            bail!("{}: {}", status, response_text);
        } else if status.is_server_error() {
            if matches!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE)
                && response_text.contains("No testruns available")
            {
                return Ok(None);
            } else {
                bail!("{}: {}", status, response_text);
            }
        }

        serde_json::from_str(&response_text)
            .map(|testrun| {
                tracing::info!("Received testrun assignment: {:?}", testrun);
                testrun
            })
            .map_err(|err| {
                tracing::error!("err");
                err.into()
            })
    }

    #[instrument(level = "debug", skip(self, probe_result))]
    pub async fn submit_results(
        &self,
        testrun_id: i64,
        probe_result: String,
        assigned_at_utc: i64,
    ) -> anyhow::Result<()> {
        let target_url = self.api.submit_results(testrun_id);

        let payload = submit_results::Payload {
            probe_result,
            agent_public_key: self.auth_key.public_key(),
            assigned_at_utc,
        };
        let signature = self.sign_message(&payload)?;
        let submit_results = submit_results::SubmitResults { payload, signature };

        let res = self
            .client
            .post(target_url)
            .json(&submit_results)
            .send()
            .await
            .and_then(|response| response.error_for_status())?;

        tracing::debug!("Submitted results: {})", res.status());
        Ok(())
    }

    #[instrument(level = "debug", skip(self, probe_result))]
    pub async fn submit_results_with_context(
        &self,
        testrun_id: i32,
        probe_result: String,
        assigned_at_utc: i64,
        gateway_identity_key: String,
    ) -> anyhow::Result<()> {
        let target_url = self.api.submit_results_v2(testrun_id);

        let payload = submit_results_v2::Payload {
            probe_result,
            agent_public_key: self.auth_key.public_key(),
            assigned_at_utc,
            gateway_identity_key,
        };
        let signature = self.sign_message(&payload)?;
        let submit_results = submit_results_v2::SubmitResultsV2 { payload, signature };

        let res = self
            .client
            .post(target_url)
            .json(&submit_results)
            .send()
            .await
            .and_then(|response| response.error_for_status())?;

        tracing::debug!("Submitted results with context: {})", res.status());
        Ok(())
    }

    fn sign_message<T>(&self, message: &T) -> anyhow::Result<Signature>
    where
        T: serde::Serialize,
    {
        let serialized = bincode::serialize(message)?;
        let signed = self.auth_key.sign(&serialized);
        Ok(signed)
    }
}
