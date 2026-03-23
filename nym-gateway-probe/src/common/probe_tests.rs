// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common::helpers::mixnet_debug_config;
use crate::common::nodes::{TestedNodeDetails, TestedNodeLpDetails};
use crate::common::socks5_test::HttpsConnectivityTest;
use crate::common::types::{
    Entry, Exit, IpPingReplies, LpProbeResults, ProbeOutcome, Socks5ProbeResults, WgProbeResults,
};
use crate::common::wireguard::{
    TwoHopWgTunnelConfig, WgTunnelConfig, run_tunnel_tests, run_two_hop_tunnel_tests,
};
use crate::common::{helpers, icmp};
use crate::config::{NetstackArgs, Socks5Args};
use anyhow::bail;
use base64::{Engine, engine::general_purpose};
use bytes::BytesMut;
use futures::StreamExt;
use nym_authenticator_client::AuthenticatorClient;
use nym_authenticator_requests::{
    AuthenticatorVersion, client_message::ClientMessage, response::AuthenticatorResponse, v2, v3,
    v4, v5, v6,
};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_config::defaults::mixnet_vpn::{NYM_TUN_DEVICE_ADDRESS_V4, NYM_TUN_DEVICE_ADDRESS_V6};
use nym_connection_monitor::self_ping_and_wait;
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_ip_packet_client::IprClientConnect;
use nym_ip_packet_requests::{IpPair, codec::MultiIpPacketCodec};
use nym_lp::peer::DHKeyPair;
use nym_registration_client::{LpRegistrationClient, NestedLpSession};
use nym_sdk::NymNetworkDetails;
use nym_sdk::mixnet::{MixnetClient, MixnetClientBuilder, NodeIdentity, Recipient, Socks5};
use nym_topology::{HardcodedTopologyProvider, NymTopology};
use rand09::SeedableRng;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_util::{codec::Decoder, sync::CancellationToken};
use tracing::*;

pub async fn wg_probe(
    mut auth_client: AuthenticatorClient,
    gateway_ip: IpAddr,
    auth_version: AuthenticatorVersion,
    awg_args: Option<String>,
    netstack_args: NetstackArgs,
    // TODO: update type
    credential: CredentialSpendingData,
) -> anyhow::Result<WgProbeResults> {
    info!("attempting to use authenticator version {auth_version:?}");

    let mut rng = rand::thread_rng();

    // that's a long conversion chain
    // (it should be simplified later...)
    // nym x25519 -> dalek x25519 -> wireguard wrapper x25519
    let private_key = nym_crypto::asymmetric::encryption::PrivateKey::new(&mut rng);
    let public_key = private_key.public_key();

    let authenticator_pub_key = public_key.inner().into();
    let init_message = match auth_version {
        AuthenticatorVersion::V2 => ClientMessage::Initial(Box::new(
            v2::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V3 => ClientMessage::Initial(Box::new(
            v3::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V4 => ClientMessage::Initial(Box::new(
            v4::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V5 => ClientMessage::Initial(Box::new(
            v5::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V6 => ClientMessage::Initial(Box::new(
            v6::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V1 | AuthenticatorVersion::UNKNOWN => bail!("unknown version number"),
    };

    let mut wg_outcome = WgProbeResults::default();

    info!(
        "connecting to authenticator: {}...",
        auth_client.auth_recipient
    );
    let response = auth_client
        .send_and_wait_for_response(&init_message)
        .await?;

    let registered_data = match response {
        AuthenticatorResponse::PendingRegistration(pending_registration_response) => {
            // Unwrap since we have already checked that we have the keypair.
            debug!("Verifying data");
            pending_registration_response.verify(&private_key)?;

            let credential = credential
                .try_into()
                .inspect_err(|err| error!("invalid zk-nym data: {err}"))
                .ok();

            let finalized_message =
                pending_registration_response.finalise_registration(&private_key, credential);
            let client_message = ClientMessage::Final(finalized_message);

            let response = auth_client
                .send_and_wait_for_response(&client_message)
                .await?;
            let AuthenticatorResponse::Registered(registered_response) = response else {
                bail!("Unexpected response");
            };
            registered_response
        }
        AuthenticatorResponse::Registered(registered_response) => registered_response,
        _ => bail!("Unexpected response"),
    };

    let peer_public = registered_data.pub_key().inner();
    let public_key_bs64 = general_purpose::STANDARD.encode(peer_public.as_bytes());
    let private_key_hex = hex::encode(private_key.to_bytes());
    let public_key_hex = hex::encode(peer_public.as_bytes());

    info!("WG connection details");
    info!("Peer public key: {}", public_key_bs64);
    info!(
        "ips {}(v4) {}(v6), port {}",
        registered_data.private_ips().ipv4,
        registered_data.private_ips().ipv6,
        registered_data.wg_port(),
    );

    let wg_endpoint = format!("{gateway_ip}:{}", registered_data.wg_port());

    info!("Successfully registered with the gateway");

    wg_outcome.can_register = true;

    // Run tunnel connectivity tests using shared helper
    let tunnel_config = WgTunnelConfig::new(
        registered_data.private_ips().ipv4.to_string(),
        registered_data.private_ips().ipv6.to_string(),
        private_key_hex,
        public_key_hex,
        wg_endpoint,
    );

    run_tunnel_tests(
        &tunnel_config,
        &netstack_args,
        &awg_args.unwrap_or_default(),
        &mut wg_outcome,
    );

    Ok(wg_outcome)
}

pub async fn lp_registration_probe(
    gateway_identity: NodeIdentity,
    gateway_lp_data: TestedNodeLpDetails,
    bandwidth_controller: &dyn BandwidthTicketProvider,
) -> anyhow::Result<LpProbeResults> {
    let lp_address = gateway_lp_data.address;
    let lp_version = gateway_lp_data.lp_version;
    let lp_ciphersuite = gateway_lp_data.ciphersuite;
    let peer = gateway_lp_data.into_remote_peer();

    info!("Starting LP registration probe for gateway at {lp_address}");

    let mut lp_outcome = LpProbeResults::default();

    // Generate Ed25519 keypair for this connection (X25519 will be derived internally by LP)
    let mut rng09 = rand09::rngs::StdRng::from_os_rng();
    let client_x25519_keypair = Arc::new(DHKeyPair::new(&mut rng09));

    // Step 0: Derive X25519 keys from Ed25519 for the gateways

    // Create LP registration client (uses Ed25519 keys directly, derives X25519 internally)
    let mut client = LpRegistrationClient::<TcpStream>::new_with_default_config(
        client_x25519_keypair,
        peer,
        lp_address,
        lp_ciphersuite,
        lp_version,
    );

    // Step 1: Perform handshake (connection is implicit in packet-per-connection model)
    // LpRegistrationClient uses packet-per-connection model - connect() is gone,
    // connection is established during handshake and registration automatically.
    info!("Performing LP handshake at {lp_address}...");
    match client.perform_handshake().await {
        Ok(_) => {
            info!("LP handshake completed successfully");
            lp_outcome.can_connect = true; // Connection succeeded if handshake succeeded
            lp_outcome.can_handshake = true;
        }
        Err(e) => {
            let error_msg = format!("LP handshake failed: {}", e);
            error!("{}", error_msg);
            lp_outcome.error = Some(error_msg);
            return Ok(lp_outcome);
        }
    }

    // Step 2: Register with gateway (send request + receive response in one call)
    info!("Sending LP registration request...");

    // Generate WireGuard keypair for dVPN registration
    let mut rng = rand::thread_rng();
    let wg_keypair = nym_crypto::asymmetric::x25519::KeyPair::new(&mut rng);

    // Convert gateway identity to ed25519 public key
    let gateway_ed25519_pubkey = gateway_identity;

    // Register using the new packet-per-connection API (returns GatewayData directly)
    let ticket_type = TicketType::V1WireguardEntry;
    let gateway_data = match client
        .register_dvpn(
            &mut rng09,
            &wg_keypair,
            &gateway_ed25519_pubkey,
            bandwidth_controller,
            ticket_type,
        )
        .await
    {
        Ok(data) => data,
        Err(e) => {
            let error_msg = format!("LP registration failed: {}", e);
            error!("{}", error_msg);
            lp_outcome.error = Some(error_msg);
            return Ok(lp_outcome);
        }
    };

    info!("LP registration successful! Received gateway data:");
    info!("  - Gateway public key: {:?}", gateway_data.public_key);
    info!(
        "  - PSK: {:?}",
        gateway_data
            .psk
            .map(|k| general_purpose::STANDARD.encode(k.as_bytes()))
    );
    info!("  - Private IPv4: {}", gateway_data.private_ipv4);
    info!("  - Private IPv6: {}", gateway_data.private_ipv6);
    info!("  - Endpoint: {}", gateway_data.endpoint);
    lp_outcome.can_register = true;

    Ok(lp_outcome)
}

/// LP-based WireGuard probe: Tests LP nested session registration + WireGuard tunnel connectivity
///
/// This function tests the full VPN flow using LP registration instead of mixnet+authenticator:
/// 1. Connects to entry gateway (outer LP session)
/// 2. Registers with exit gateway via entry forwarding (nested LP session)
/// 3. Receives WireGuard configuration from both gateways
/// 4. Tests WireGuard tunnel connectivity (IPv4/IPv6)
///
/// This validates that IP hiding works (exit sees entry IP, not client IP) and that the
/// full VPN tunnel operates correctly after LP registration.
///
// Known issue in localnet mode - After this probe runs, container networking
// to the external internet becomes unstable while internal container-to-container traffic
// continues to work. The two-hop WireGuard tunnel itself succeeds (handshake completes),
// but subsequent DNS/ping tests may timeout. This appears to be related to Apple Container
// Runtime networking quirks combined with our NAT/iptables configuration. Tracked in
// beads issue nym-vbdo. Workaround: restart the localnet containers between probe runs.
pub async fn wg_probe_lp(
    entry_gateway: &TestedNodeDetails,
    exit_gateway: &TestedNodeDetails,
    bandwidth_controller: &dyn BandwidthTicketProvider,
    awg_args: Option<String>,
    netstack_args: NetstackArgs,
) -> anyhow::Result<WgProbeResults> {
    // Validate that both gateways have required information
    let entry_lp_data = entry_gateway
        .lp_data
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Entry gateway missing LP data"))?;

    let exit_lp_data = exit_gateway
        .lp_data
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Exit gateway missing LP data"))?;

    let entry_address = entry_lp_data.address;
    let exit_address = exit_lp_data.address;

    let entry_lp_version = entry_lp_data.lp_version;
    let exit_lp_version = exit_lp_data.lp_version;

    let entry_lp_ciphersuite = entry_lp_data.ciphersuite;
    let exit_lp_ciphersuite = exit_lp_data.ciphersuite;

    info!("Starting LP-based WireGuard probe (entry→exit via forwarding)");

    let mut wg_outcome = WgProbeResults::default();

    // Generate x25519 keypairs for LP protocol
    let mut rng09 = rand09::rngs::StdRng::from_os_rng();

    let entry_lp_keypair = Arc::new(DHKeyPair::new(&mut rng09));
    let exit_lp_keypair = Arc::new(DHKeyPair::new(&mut rng09));

    // Generate WireGuard keypairs for VPN registration
    let mut rng = rand::rngs::OsRng;
    let entry_wg_keypair = x25519::KeyPair::new(&mut rng);
    let exit_wg_keypair = x25519::KeyPair::new(&mut rng);

    let entry_peer = entry_lp_data.into_remote_peer();
    let exit_peer = exit_lp_data.into_remote_peer();

    // STEP 1: Establish outer LP session with entry gateway
    // LpRegistrationClient uses packet-per-connection model - connect() is gone,
    // connection is established automatically during handshake.
    info!("Establishing outer LP session with entry gateway...");
    let mut entry_client = LpRegistrationClient::<TcpStream>::new_with_default_config(
        entry_lp_keypair,
        entry_peer,
        entry_address,
        entry_lp_ciphersuite,
        entry_lp_version,
    );

    // Perform handshake with entry gateway (connection is implicit)
    if let Err(e) = entry_client.perform_handshake().await {
        error!("Failed to handshake with entry gateway: {}", e);
        return Ok(wg_outcome);
    }
    info!("Outer LP session with entry gateway established");

    // STEP 2: Use nested session to register with exit gateway via forwarding
    info!("Registering with exit gateway via entry forwarding...");
    let mut nested_session = NestedLpSession::new(
        exit_address,
        exit_lp_keypair,
        exit_peer,
        exit_lp_ciphersuite,
        exit_lp_version,
    );

    let exit_gateway_pubkey = exit_gateway.identity;

    // Perform handshake and registration with exit gateway via forwarding
    if let Err(err) = nested_session.perform_handshake(&mut entry_client).await {
        error!("Failed to perform handshake with exit gateway: {err}");
        return Ok(wg_outcome);
    };

    let exit_gateway_data = match nested_session
        .register_dvpn(
            &mut entry_client,
            &mut rng09,
            &exit_wg_keypair,
            &exit_gateway_pubkey,
            bandwidth_controller,
            TicketType::V1WireguardExit,
        )
        .await
    {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to register with exit gateway: {}", e);
            return Ok(wg_outcome);
        }
    };

    info!("Exit gateway registration successful via forwarding");

    // STEP 3: Register with entry gateway
    info!("Registering with entry gateway...");
    let entry_gateway_pubkey =
        ed25519::PublicKey::from_bytes(&entry_gateway.identity.to_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid entry gateway identity: {}", e))?;

    // Use packet-per-connection register() which returns GatewayData directly
    let entry_gateway_data = match entry_client
        .register_dvpn(
            &mut rng09,
            &entry_wg_keypair,
            &entry_gateway_pubkey,
            bandwidth_controller,
            TicketType::V1WireguardEntry,
        )
        .await
    {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to register with entry gateway: {}", e);
            return Ok(wg_outcome);
        }
    };
    info!("Entry gateway registration successful");

    info!("LP registration successful for both gateways!");
    wg_outcome.can_register = true;

    // STEP 4: Test WireGuard tunnels using two-hop configuration
    // Traffic flows: Exit tunnel -> UDP Forwarder -> Entry tunnel -> Exit Gateway -> Internet
    // The exit gateway endpoint is not directly reachable from the host in localnet.
    // We must tunnel through the entry gateway using the UDP forwarder pattern.

    // Convert keys to hex for netstack
    let entry_private_key_hex = hex::encode(entry_wg_keypair.private_key().to_bytes());
    let entry_public_key_hex = hex::encode(entry_gateway_data.public_key.to_bytes());
    let exit_private_key_hex = hex::encode(exit_wg_keypair.private_key().to_bytes());
    let exit_public_key_hex = hex::encode(exit_gateway_data.public_key.to_bytes());

    // Build WireGuard endpoint addresses
    // Entry endpoint uses entry_ip (host-reachable) + port from registration
    let entry_wg_endpoint = entry_gateway_data.endpoint;
    // Exit endpoint uses exit_ip + port from registration (forwarded via entry)
    let exit_wg_endpoint = exit_gateway_data.endpoint;

    info!("Two-hop WireGuard configuration:");
    info!("  Entry gateway:");
    info!("    Private IPv4: {}", entry_gateway_data.private_ipv4);
    info!("    Endpoint: {}", entry_wg_endpoint);
    info!("  Exit gateway:");
    info!("    Private IPv4: {}", exit_gateway_data.private_ipv4);
    info!("    Endpoint (via forwarder): {}", exit_wg_endpoint);

    // Build two-hop tunnel configuration
    let two_hop_config = TwoHopWgTunnelConfig::new(
        entry_gateway_data.private_ipv4.to_string(),
        entry_private_key_hex,
        entry_public_key_hex,
        entry_wg_endpoint,
        awg_args.clone().unwrap_or_default(), // Entry AWG args
        exit_gateway_data.private_ipv4.to_string(),
        exit_private_key_hex,
        exit_public_key_hex,
        exit_wg_endpoint,
        awg_args.unwrap_or_default(), // Exit AWG args
    );

    // Run two-hop tunnel connectivity tests
    run_two_hop_tunnel_tests(&two_hop_config, &netstack_args, &mut wg_outcome);

    info!("LP-based two-hop WireGuard probe completed");
    Ok(wg_outcome)
}

pub async fn do_ping(
    mut mixnet_client: MixnetClient,
    our_address: Recipient,
    exit_router_address: Option<Recipient>,
    tested_entry: bool,
) -> (anyhow::Result<ProbeOutcome>, MixnetClient) {
    let entry = do_ping_entry(&mut mixnet_client, our_address, tested_entry).await;

    let (exit_result, mixnet_client) = if let Some(exit_router_address) = exit_router_address {
        let (maybe_ip_pair, mut mixnet_client) =
            connect_exit(mixnet_client, exit_router_address).await;
        match maybe_ip_pair {
            Some(ip_pair) => (
                do_ping_exit(&mut mixnet_client, ip_pair, exit_router_address).await,
                mixnet_client,
            ),
            None => (Ok(Some(Exit::fail_to_connect())), mixnet_client),
        }
    } else {
        (Ok(None), mixnet_client)
    };

    (
        exit_result.map(|exit| ProbeOutcome {
            as_entry: entry,
            as_exit: exit,
            socks5: None,
            wg: None,
            lp: None,
        }),
        mixnet_client,
    )
}

async fn do_ping_entry(
    mixnet_client: &mut MixnetClient,
    our_address: Recipient,
    tested_entry: bool,
) -> Entry {
    // Step 1: confirm that the entry gateway is routing our mixnet traffic
    info!("Sending mixnet ping to ourselves to verify mixnet connection");

    if self_ping_and_wait(our_address, mixnet_client)
        .await
        .is_err()
    {
        return if tested_entry {
            Entry::fail_to_connect()
        } else {
            Entry::EntryFailure
        };
    }
    info!("Successfully mixnet pinged ourselves");

    Entry::success()
}

async fn connect_exit(
    mixnet_client: MixnetClient,
    exit_router_address: Recipient,
) -> (Option<IpPair>, MixnetClient) {
    // Step 2: connect to the exit gateway
    info!(
        "Connecting to exit gateway: {}",
        exit_router_address.gateway().to_base58_string()
    );
    // The IPR supports cancellation, but it's unused in the gateway probe
    let cancel_token = CancellationToken::new();
    let mut ipr_client = IprClientConnect::new(mixnet_client, cancel_token);

    let maybe_ip_pair = ipr_client.connect(exit_router_address).await;
    let mixnet_client = ipr_client.into_mixnet_client();

    if let Ok(our_ips) = maybe_ip_pair {
        info!("Successfully connected to exit gateway");
        info!("Using mixnet VPN IP addresses: {our_ips}");
        (Some(our_ips), mixnet_client)
    } else {
        (None, mixnet_client)
    }
}

pub async fn do_ping_exit(
    mixnet_client: &mut MixnetClient,
    our_ips: IpPair,
    exit_router_address: Recipient,
) -> anyhow::Result<Option<Exit>> {
    // Step 3: perform ICMP connectivity checks for the exit gateway
    send_icmp_pings(mixnet_client, our_ips, exit_router_address).await?;
    listen_for_icmp_ping_replies(mixnet_client, our_ips).await
}

async fn send_icmp_pings(
    mixnet_client: &MixnetClient,
    our_ips: IpPair,
    exit_router_address: Recipient,
) -> anyhow::Result<()> {
    // ipv4 addresses for testing
    let ipr_tun_ip_v4 = NYM_TUN_DEVICE_ADDRESS_V4;
    let external_ip_v4 = Ipv4Addr::new(8, 8, 8, 8);

    // ipv6 addresses for testing
    let ipr_tun_ip_v6 = NYM_TUN_DEVICE_ADDRESS_V6;
    let external_ip_v6 = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888);

    info!(
        "Sending ICMP echo requests to: {ipr_tun_ip_v4}, {ipr_tun_ip_v6}, {external_ip_v4}, {external_ip_v6}"
    );

    // send ipv4 pings
    for ii in 0..10 {
        icmp::send_ping_v4(
            mixnet_client,
            our_ips,
            ii,
            ipr_tun_ip_v4,
            exit_router_address,
        )
        .await?;
        icmp::send_ping_v4(
            mixnet_client,
            our_ips,
            ii,
            external_ip_v4,
            exit_router_address,
        )
        .await?;
    }

    // send ipv6 pings
    for ii in 0..10 {
        icmp::send_ping_v6(
            mixnet_client,
            our_ips,
            ii,
            ipr_tun_ip_v6,
            exit_router_address,
        )
        .await?;
        icmp::send_ping_v6(
            mixnet_client,
            our_ips,
            ii,
            external_ip_v6,
            exit_router_address,
        )
        .await?;
    }
    Ok(())
}

pub async fn listen_for_icmp_ping_replies(
    mixnet_client: &mut MixnetClient,
    our_ips: IpPair,
) -> anyhow::Result<Option<Exit>> {
    let mut multi_ip_packet_decoder = MultiIpPacketCodec::new();
    let mut registered_replies = IpPingReplies::new();

    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                info!("Finished waiting for ICMP echo reply from exit gateway");
                break;
            }
            Some(reconstructed_message) = mixnet_client.next() => {
                let Some(data_response) = helpers::unpack_data_response(&reconstructed_message) else {
                    continue;
                };

                // IP packets are bundled together in a mixnet message
                let mut bytes = BytesMut::from(&*data_response.ip_packet);
                while let Ok(Some(packet)) = multi_ip_packet_decoder.decode(&mut bytes) {
                    if let Some(event) = icmp::check_for_icmp_beacon_reply(&packet.into_bytes(), icmp::icmp_identifier(), our_ips) {
                        info!("Received ICMP echo reply from exit gateway");
                        info!("Connection event: {event:?}");
                        registered_replies.register_event(&event);
                    }
                }
            }
        }
    }

    Ok(Some(Exit {
        can_connect: true,
        can_route_ip_v4: registered_replies.ipr_tun_ip_v4,
        can_route_ip_external_v4: registered_replies.external_ip_v4,
        can_route_ip_v6: registered_replies.ipr_tun_ip_v6,
        can_route_ip_external_v6: registered_replies.external_ip_v6,
    }))
}

/// Creates a SOCKS5 proxy connection through the mixnet to the exit GW
/// and performs necessary tests.
#[instrument(level = "info", name = "socks5_test", skip_all)]
pub(crate) async fn do_socks5_connectivity_test(
    nr_recipient: &Recipient,
    entry_gateway_id: NodeIdentity,
    network_details: NymNetworkDetails,
    min_gw_performance: Option<u8>,
    socks5_args: Socks5Args,
    maybe_topology: Option<NymTopology>,
) -> anyhow::Result<Socks5ProbeResults> {
    info!(
        "Starting SOCKS5 test through Network Requester: {}",
        nr_recipient
    );
    if socks5_args.socks5_json_rpc_url_list.is_empty() {
        bail!("You need to define JSON RPC URLs in order to test SOCKS5")
    }

    info!(
        "Network Requester gateway: {}",
        nr_recipient.gateway().to_base58_string()
    );
    info!(
        "Network Requester identity: {}",
        nr_recipient.identity().to_base58_string()
    );

    // create ephemeral SOCKS5 client
    let socks5_config = Socks5::new(nr_recipient.to_string());

    // debug config similar to main probe
    let debug_config = mixnet_debug_config(min_gw_performance, true);

    let mut socks5_client_builder = MixnetClientBuilder::new_ephemeral()
        // Specify entry gateway explicitly
        .request_gateway(entry_gateway_id.to_base58_string())
        .socks5_config(socks5_config)
        .network_details(network_details)
        .debug_config(debug_config);

    if let Some(topology) = maybe_topology {
        socks5_client_builder = socks5_client_builder
            .custom_topology_provider(Box::new(HardcodedTopologyProvider::new(topology)));
    }

    let disconnected_socks5_client = socks5_client_builder.build()?;

    // connect to mixnet via SOCKS5
    let socks5_client = match disconnected_socks5_client
        .connect_to_mixnet_via_socks5()
        .await
    {
        Ok(client) => {
            info!("🌐 Successfully connected to mixnet via SOCKS5 proxy");
            info!(
                "Connected via entry gateway: {}",
                client.nym_address().gateway().to_base58_string()
            );
            client
        }
        Err(e) => {
            error!("Failed to establish SOCKS5 connection: {}", e);
            return Ok(Socks5ProbeResults::error_before_connecting(format!(
                "SOCKS5 connection failed: {}",
                e
            )));
        }
    };

    let test = match HttpsConnectivityTest::new(
        socks5_args.test_count,
        socks5_args.mixnet_client_timeout_sec,
        socks5_args.failure_count_cutoff,
        socks5_args.socks5_json_rpc_url_list,
        socks5_client.socks5_url(),
    ) {
        Ok(test) => test,
        Err(err) => {
            socks5_client.disconnect().await;

            error!("{err}");
            return Ok(Socks5ProbeResults::error_after_connecting(
                "Failed to create client",
            ));
        }
    };

    let result = test.run_tests().await;
    socks5_client.disconnect().await;

    Ok(Socks5ProbeResults::with_http_result(result))
}
