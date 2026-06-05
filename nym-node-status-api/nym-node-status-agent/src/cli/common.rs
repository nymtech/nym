use crate::cli::{ServerConfig, parse_server_config};
use anyhow::anyhow;
use nym_gateway_probe::config::CredentialArgs;
use nym_gateway_probe::types::{AttachedTicketMaterials, VersionedSerialise};
use nym_node_status_client::NsApiClient;
use nym_sdk::mixnet::ed25519::PublicKey;

pub(crate) fn parse_servers(raw: &[String]) -> anyhow::Result<Vec<ServerConfig>> {
    raw.iter()
        .map(|s| {
            parse_server_config(s).map_err(|e| {
                tracing::error!("Invalid server config '{}': {}", s, e);
                anyhow!("Invalid server config '{}': {}", s, e)
            })
        })
        .collect()
}

pub(crate) fn primary(servers: &[ServerConfig]) -> anyhow::Result<&ServerConfig> {
    servers
        .first()
        .ok_or_else(|| anyhow!("No servers configured"))
}

pub(crate) fn build_client(server: &ServerConfig) -> NsApiClient {
    let auth_key =
        nym_crypto::asymmetric::ed25519::PrivateKey::from_bytes(&server.auth_key.to_bytes())
            .expect("Failed to clone auth key");
    NsApiClient::new(&server.address, server.port, auth_key)
}

pub(crate) fn parse_gateway_pubkey(key: &str) -> anyhow::Result<PublicKey> {
    PublicKey::from_base58_string(key).map_err(|e| anyhow!("Failed to parse GW identity key: {e}"))
}

pub(crate) fn credential_args_from(materials: AttachedTicketMaterials) -> CredentialArgs {
    CredentialArgs {
        ticket_materials: materials.to_serialised_string(),
        ticket_materials_revision:
            <AttachedTicketMaterials as VersionedSerialise>::CURRENT_SERIALISATION_REVISION,
    }
}
