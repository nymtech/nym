// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::LocalWireguardOpts;
use crate::config::Config;
use clap::crate_version;
use nym_gateway::node::{
    LocalAuthenticatorOpts, LocalIpPacketRouterOpts, LocalNetworkRequesterOpts,
};
use nym_gateway::nym_authenticator;

// a temporary solution until further refactoring is made
fn ephemeral_gateway_config(config: &Config) -> nym_gateway::config::Config {
    nym_gateway::config::Config::new(
        nym_gateway::config::Gateway {
            enforce_zk_nyms: config.gateway_tasks.enforce_zk_nyms,
            websocket_bind_address: config.gateway_tasks.ws_bind_address,
            nym_api_urls: config.mixnet.nym_api_urls.clone(),
            nyxd_urls: config.mixnet.nyxd_urls.clone(),
        },
        nym_gateway::config::NetworkRequester {
            enabled: config.service_providers.ip_packet_router.debug.enabled,
        },
        nym_gateway::config::IpPacketRouter {
            enabled: config.service_providers.network_requester.debug.enabled,
        },
        nym_gateway::config::Debug {
            client_bandwidth_max_flushing_rate: config
                .gateway_tasks
                .debug
                .client_bandwidth
                .max_flushing_rate,
            client_bandwidth_max_delta_flushing_amount: config
                .gateway_tasks
                .debug
                .client_bandwidth
                .max_delta_flushing_amount,
            stale_messages_cleaner_run_interval: config
                .gateway_tasks
                .debug
                .stale_messages
                .cleaner_run_interval,
            stale_messages_max_age: config.gateway_tasks.debug.stale_messages.max_age,
            maximum_open_connections: config.gateway_tasks.debug.maximum_open_connections,
            zk_nym_tickets: nym_gateway::config::ZkNymTicketHandlerDebug {
                revocation_bandwidth_penalty: config
                    .gateway_tasks
                    .debug
                    .zk_nym_tickets
                    .revocation_bandwidth_penalty,
                pending_poller: config.gateway_tasks.debug.zk_nym_tickets.pending_poller,
                minimum_api_quorum: config.gateway_tasks.debug.zk_nym_tickets.minimum_api_quorum,
                minimum_redemption_tickets: config
                    .gateway_tasks
                    .debug
                    .zk_nym_tickets
                    .minimum_redemption_tickets,
                maximum_time_between_redemption: config
                    .gateway_tasks
                    .debug
                    .zk_nym_tickets
                    .maximum_time_between_redemption,
            },
            max_request_timestamp_skew: config.gateway_tasks.debug.max_request_timestamp_skew,
        },
    )
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

pub struct GatewayTasksConfig {
    pub gateway: nym_gateway::config::Config,
    pub nr_opts: Option<LocalNetworkRequesterOpts>,
    pub ipr_opts: Option<LocalIpPacketRouterOpts>,
    pub auth_opts: Option<LocalAuthenticatorOpts>,
    #[allow(dead_code)]
    pub wg_opts: LocalWireguardOpts,
}

// that function is rather disgusting, but I hope it's not going to live for too long
pub fn gateway_tasks_config(config: &Config) -> GatewayTasksConfig {
    let mut nr_opts = LocalNetworkRequesterOpts {
        config: nym_network_requester::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(config),
                debug: config
                    .service_providers
                    .network_requester
                    .debug
                    .client_debug,
            },
            network_requester: nym_network_requester::config::NetworkRequester {
                open_proxy: config.service_providers.open_proxy,
                disable_poisson_rate: config
                    .service_providers
                    .network_requester
                    .debug
                    .disable_poisson_rate,
                upstream_exit_policy_url: Some(
                    config.service_providers.upstream_exit_policy_url.clone(),
                ),
            },
            storage_paths: nym_network_requester::config::NetworkRequesterPaths {
                common_paths: config
                    .service_providers
                    .storage_paths
                    .network_requester
                    .to_common_client_paths(),
            },
            network_requester_debug: Default::default(),
            logging: config.logging,
        },
        custom_mixnet_path: None,
    };

    // SAFETY: this function can only fail if fastmode or nocover is set alongside medium_toggle which is not the case here
    #[allow(clippy::unwrap_used)]
    nr_opts
        .config
        .base
        .try_apply_traffic_modes(
            nr_opts.config.network_requester.disable_poisson_rate,
            false,
            false,
            false,
        )
        .unwrap();

    let mut ipr_opts = LocalIpPacketRouterOpts {
        config: nym_ip_packet_router::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(config),
                debug: config.service_providers.ip_packet_router.debug.client_debug,
            },
            ip_packet_router: nym_ip_packet_router::config::IpPacketRouter {
                disable_poisson_rate: config
                    .service_providers
                    .ip_packet_router
                    .debug
                    .disable_poisson_rate,
                upstream_exit_policy_url: Some(
                    config.service_providers.upstream_exit_policy_url.clone(),
                ),
            },
            storage_paths: nym_ip_packet_router::config::IpPacketRouterPaths {
                common_paths: config
                    .service_providers
                    .storage_paths
                    .ip_packet_router
                    .to_common_client_paths(),
                ip_packet_router_description: Default::default(),
            },

            logging: config.logging,
        },
        custom_mixnet_path: None,
    };

    if ipr_opts.config.ip_packet_router.disable_poisson_rate {
        ipr_opts.config.base.set_no_poisson_process()
    }

    let mut auth_opts = LocalAuthenticatorOpts {
        config: nym_authenticator::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(config),
                debug: config.service_providers.authenticator.debug.client_debug,
            },
            authenticator: config.wireguard.clone().into(),
            storage_paths: nym_authenticator::config::AuthenticatorPaths {
                common_paths: config
                    .service_providers
                    .storage_paths
                    .authenticator
                    .to_common_client_paths(),
            },
        },
        custom_mixnet_path: None,
    };

    if config
        .service_providers
        .authenticator
        .debug
        .disable_poisson_rate
    {
        auth_opts.config.base.set_no_poisson_process();
    }

    let wg_opts = LocalWireguardOpts {
        config: super::Wireguard {
            enabled: config.wireguard.enabled,
            bind_address: config.wireguard.bind_address,
            private_ipv4: config.wireguard.private_ipv4,
            private_ipv6: config.wireguard.private_ipv6,
            announced_port: config.wireguard.announced_port,
            private_network_prefix_v4: config.wireguard.private_network_prefix_v4,
            private_network_prefix_v6: config.wireguard.private_network_prefix_v6,
            storage_paths: config.wireguard.storage_paths.clone(),
        },
        custom_mixnet_path: None,
    };

    GatewayTasksConfig {
        gateway: ephemeral_gateway_config(config),
        nr_opts: Some(nr_opts),
        ipr_opts: Some(ipr_opts),
        auth_opts: Some(auth_opts),
        wg_opts,
    }
}
