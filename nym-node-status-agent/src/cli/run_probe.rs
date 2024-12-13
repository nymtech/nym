use anyhow::{bail, Context};
use nym_common_models::ns_api::{get_testrun, submit_results, TestrunAssignment};
use nym_crypto::asymmetric::ed25519::{PrivateKey, Signature};
use std::fmt::Display;
use tracing::instrument;

use crate::cli::GwProbe;

const INTERNAL_TESTRUNS: &str = "internal/testruns";

pub(crate) async fn run_probe(
    server_ip: &str,
    server_port: u16,
    ns_api_auth_key: &str,
    probe_path: &str,
) -> anyhow::Result<()> {
    let auth_key = PrivateKey::from_base58_string(ns_api_auth_key)
        .context("Couldn't parse auth key, exiting")?;
    let ns_api_client = Client::new(server_ip, server_port, auth_key);

    let probe = GwProbe::new(probe_path.to_string());

    let version = probe.version().await;
    tracing::info!("Probe version:\n{}", version);

    if let Some(testrun) = ns_api_client.request_testrun().await? {
        let log = probe.run_and_get_log(&Some(testrun.gateway_identity_key));

        ns_api_client
            .submit_results(testrun.testrun_id, log, testrun.assigned_at_utc)
            .await?;
    } else {
        tracing::info!("No testruns available, exiting")
    }

    Ok(())
}

struct Client {
    server_address: String,
    client: reqwest::Client,
    auth_key: PrivateKey,
}

impl Client {
    pub fn new(server_ip: &str, server_port: u16, auth_key: PrivateKey) -> Self {
        let server_address = format!("{}:{}", server_ip, server_port);
        let client = reqwest::Client::new();

        Self {
            server_address,
            client,
            auth_key,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub(crate) async fn request_testrun(&self) -> anyhow::Result<Option<TestrunAssignment>> {
        let target_url = self.api_with_subpath(None::<String>);

        let payload = get_testrun::Payload {
            agent_public_key: self.auth_key.public_key(),
            timestamp: chrono::offset::Utc::now().timestamp(),
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
    pub(crate) async fn submit_results(
        &self,
        testrun_id: i64,
        probe_result: String,
        assigned_at_utc: i64,
    ) -> anyhow::Result<()> {
        let target_url = self.api_with_subpath(Some(testrun_id));

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

    fn sign_message<T>(&self, message: &T) -> anyhow::Result<Signature>
    where
        T: serde::Serialize,
    {
        let serialized = bincode::serialize(message)?;
        let signed = self.auth_key.sign(&serialized);
        Ok(signed)
    }

    fn api_with_subpath(&self, subpath: Option<impl Display>) -> String {
        if let Some(subpath) = subpath {
            format!("{}/{}/{}", self.server_address, INTERNAL_TESTRUNS, subpath)
        } else {
            format!("{}/{}", self.server_address, INTERNAL_TESTRUNS)
        }
    }
}
