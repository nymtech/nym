// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use clap::crate_version;
use nym_gateway::node::{
    LocalAuthenticatorOpts, LocalIpPacketRouterOpts, LocalNetworkRequesterOpts,
};
use std::net::IpAddr;
use thiserror::Error;

use super::LocalWireguardOpts;

#[derive(Debug, Error)]
#[error("currently it's not supported to have different ip addresses for clients and mixnet ({clients_bind_ip} and {mix_bind_ip} were used)")]
pub struct UnsupportedGatewayAddresses {
    clients_bind_ip: IpAddr,
    mix_bind_ip: IpAddr,
}

// a temporary solution until all nodes are even more tightly integrated
pub fn ephemeral_gateway_config(
    config: Config,
    mnemonic: &bip39::Mnemonic,
) -> Result<nym_gateway::config::Config, UnsupportedGatewayAddresses> {
    let host = nym_gateway::config::Host {
        public_ips: config.host.public_ips,
        hostname: config.host.hostname,
    };

    let http = nym_gateway::config::Http {
        bind_address: config.http.bind_address,
        landing_page_assets_path: config.http.landing_page_assets_path,
    };

    let clients_bind_ip = config.entry_gateway.bind_address.ip();
    let mix_bind_ip = config.mixnet.bind_address.ip();
    if clients_bind_ip != mix_bind_ip {
        return Err(UnsupportedGatewayAddresses {
            clients_bind_ip,
            mix_bind_ip,
        });
    }

    // SAFETY: we're using hardcoded valid url here (that won't be used anyway)
    #[allow(clippy::unwrap_used)]
    let gateway = nym_gateway::config::Gateway {
        // that field is very much irrelevant, but I guess let's keep them for now
        version: format!("{}-nym-node", crate_version!()),
        id: config.id,
        only_coconut_credentials: config.entry_gateway.enforce_zk_nyms,
        listening_address: clients_bind_ip,
        mix_port: config.mixnet.bind_address.port(),
        clients_port: config.entry_gateway.bind_address.port(),
        clients_wss_port: config.entry_gateway.announce_wss_port,
        nym_api_urls: config.mixnet.nym_api_urls,
        nyxd_urls: config.mixnet.nyxd_urls,

        // that's nasty but can't do anything about it for this temporary solution : (
        cosmos_mnemonic: mnemonic.clone(),
    };

    Ok(nym_gateway::config::Config::externally_loaded(
        host,
        http,
        gateway,
        nym_gateway::config::NetworkRequester { enabled: false },
        nym_gateway::config::IpPacketRouter { enabled: false },
        nym_gateway::config::Debug {
            packet_forwarding_initial_backoff: config
                .mixnet
                .debug
                .packet_forwarding_initial_backoff,
            packet_forwarding_maximum_backoff: config
                .mixnet
                .debug
                .packet_forwarding_maximum_backoff,
            initial_connection_timeout: config.mixnet.debug.initial_connection_timeout,
            maximum_connection_buffer_size: config.mixnet.debug.maximum_connection_buffer_size,
            message_retrieval_limit: config.entry_gateway.debug.message_retrieval_limit,
            use_legacy_framed_packet_version: false,
            zk_nym_tickets: nym_gateway::config::ZkNymTicketHandlerDebug {
                revocation_bandwidth_penalty: config
                    .entry_gateway
                    .debug
                    .zk_nym_tickets
                    .revocation_bandwidth_penalty,
                pending_poller: config.entry_gateway.debug.zk_nym_tickets.pending_poller,
                minimum_api_quorum: config.entry_gateway.debug.zk_nym_tickets.minimum_api_quorum,
                minimum_redemption_tickets: config
                    .entry_gateway
                    .debug
                    .zk_nym_tickets
                    .minimum_redemption_tickets,
                maximum_time_between_redemption: config
                    .entry_gateway
                    .debug
                    .zk_nym_tickets
                    .maximum_time_between_redemption,
            },
            ..Default::default()
        },
    ))
}

pub fn base_client_config(config: &Config) -> nym_client_core_config_types::Client {
    nym_client_core_config_types::Client {
        version: format!("{}-nym-node", crate_version!()),
        id: config.id.clone(),
        // irrelevant field - no need for credentials in embedded mode
        disabled_credentials_mode: true,
        nyxd_urls: config.mixnet.nyxd_urls.clone(),
        nym_api_urls: config.mixnet.nym_api_urls.clone(),
    }
}

pub struct EphemeralConfig {
    pub gateway: nym_gateway::config::Config,
    pub nr_opts: Option<LocalNetworkRequesterOpts>,
    pub ipr_opts: Option<LocalIpPacketRouterOpts>,
    pub auth_opts: LocalAuthenticatorOpts,
    pub wg_opts: LocalWireguardOpts,
}
