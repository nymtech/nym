// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use nym_bandwidth_controller::mock::MockBandwidthController;
    use nym_credential_verification::ecash::MockEcashManager;
    use nym_credential_verification::upgrade_mode::testing::mock_dummy_upgrade_mode_details;
    use nym_credentials_interface::TicketType;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_kkt::key_utils::{
        generate_keypair_mceliece, generate_keypair_mlkem, generate_lp_keypair_x25519,
    };
    use nym_kkt::keys::KEMKeys;
    use nym_kkt_ciphersuite::Ciphersuite;
    use nym_lp::peer::LpLocalPeer;
    use nym_lp::transport::mock::MockTransport;
    use nym_lp_data::packet::version;
    use nym_node::config::{LpConfig, LpDebug};
    use nym_node::node::GatewayStorage;
    use nym_node::node::lp::active_sessions::ActiveLpSessions;
    use nym_node::node::lp::control::ingress::connection_handler::LpIngressConnectionHandler;
    use nym_node::node::lp::directory::{LpNodeDetails, LpNodes};
    use nym_node::node::lp::error::LpHandlerError;
    use nym_node::node::lp::registration_nonces::RegistrationNonces;
    use nym_node::node::lp::{SharedGatewayLpControlState, SharedLpControlState};
    use nym_node::wireguard::{PeerManager, PeerRegistrator};
    use nym_registration_client::{
        LpClientError, LpDvpnRegistrationClient, LpGatewayClient, NestedLpDvpnRegistrationClient,
    };
    use nym_test_utils::helpers::CryptoRng010;
    use nym_test_utils::traits::Timeboxed;
    use nym_wireguard::peer_controller::IpPair;
    use nym_wireguard::peer_controller::mock::{
        Key, KeyWrapper, MockPeerController, MockPeerControllerState, PeerControlRequestType,
        RegisteredResponse, mock_peer_controller,
    };
    use nym_wireguard::{IpPool, PeerControlRequest, WireguardConfig, WireguardGatewayData};
    use std::collections::HashMap;
    use std::mem;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::sync::mpsc::Receiver;
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;
    use tracing::error;

    trait WgKeyConv {
        fn to_wg_key(self) -> KeyWrapper;
    }

    impl WgKeyConv for x25519::PublicKey {
        fn to_wg_key(self) -> KeyWrapper {
            KeyWrapper::from(Key::new(self.to_bytes()))
        }
    }

    struct Party {
        identity: ed25519::KeyPair,
        peer: LpLocalPeer,
        x25519_wg_keys: Arc<x25519::KeyPair>,
        socket_addr: SocketAddr,
        lp_version: u8,
    }

    impl Party {
        fn generate(rng: &mut impl CryptoRng010) -> Self {
            let mut ip = [0u8; 4];
            let mut port = [0u8; 2];

            rng.fill_bytes(&mut ip);
            rng.fill_bytes(&mut port);
            let ed25519_keys = ed25519::KeyPair::new(rng);
            let x25519_wg_keys = Arc::new(x25519::KeyPair::new(rng));

            let lp_x25519_keys = Arc::new(generate_lp_keypair_x25519(rng));
            let mlkem_keypair = generate_keypair_mlkem(rng);
            let mceliece_keypair = generate_keypair_mceliece(rng);
            let lp_kem_keys = KEMKeys::new(mceliece_keypair, mlkem_keypair);

            let ciphersuite = Ciphersuite::default();

            Party {
                identity: ed25519_keys,
                peer: LpLocalPeer::new(ciphersuite, lp_x25519_keys.clone())
                    .with_kem_keys(lp_kem_keys),
                x25519_wg_keys,
                socket_addr: SocketAddr::from((ip, u16::from_le_bytes(port))),
                lp_version: version::CURRENT,
            }
        }

        /// What the directory would publish for this party as an LP node.
        fn lp_node_details(&self) -> LpNodeDetails {
            let remote = self.peer.as_remote();
            LpNodeDetails::new(
                u32::from(self.socket_addr.port()),
                remote.kem_key_digests().clone(),
                *remote.x25519(),
                self.socket_addr.port(),
                self.socket_addr.port().wrapping_add(1),
                self.lp_version,
            )
        }
    }

    struct Client {
        base: Party,

        ticket_provider: MockBandwidthController,
    }

    impl Client {
        fn mock(rng: &mut impl CryptoRng010) -> Self {
            Client {
                base: Party::generate(rng),
                ticket_provider: Default::default(),
            }
        }
    }

    #[allow(clippy::large_enum_variant)]
    enum SpawnedPeerController {
        Ready { controller: MockPeerController },
        Running { handle: JoinHandle<Option<()>> },
        Finished,

        // needed for temporary mem replace
        Invalid,
    }

    #[allow(clippy::large_enum_variant)]
    enum SpawnedLpConnectionHandlerState {
        NotCreated,
        Ready {
            handler: LpIngressConnectionHandler<MockTransport>,
        },
        Running {
            handle: JoinHandle<Option<Result<(), LpHandlerError>>>,
        },
        Finished,
    }

    struct Gateway {
        base: Party,
        lp_state: SharedLpControlState,
        gateway_lp_state: SharedGatewayLpControlState,
        ip_pool: IpPool,
        mock_peer_controller: SpawnedPeerController,

        tasks_cancellation: CancellationToken,
        mock_peer_controller_state: MockPeerControllerState,
        lp_connection_handler: SpawnedLpConnectionHandlerState,
    }

    impl Gateway {
        async fn register_peer_controller_response(
            &self,
            request: PeerControlRequestType,
            response: impl Into<RegisteredResponse>,
        ) {
            self.mock_peer_controller_state
                .register_response(request, response)
                .await;
        }

        fn wireguard_data(base: &Party) -> (WireguardGatewayData, Receiver<PeerControlRequest>) {
            // some sensible default values (ports don't matter anyway)
            let cfg = WireguardConfig {
                bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 51822),
                private_ipv4: Ipv4Addr::new(10, 1, 0, 1),
                private_ipv6: Ipv6Addr::new(0xfc01, 0, 0, 0, 0, 0, 0, 0x1), // fc01::1,
                announced_tunnel_port: 51822,
                announced_metadata_port: 51830,
                private_network_prefix_v4: 16,
                private_network_prefix_v6: 112,
            };

            WireguardGatewayData::new(cfg, base.x25519_wg_keys.clone())
        }

        fn ip_pool() -> IpPool {
            IpPool::new(
                Ipv4Addr::new(10, 1, 0, 1),
                16,
                Ipv6Addr::new(0xfc01, 0, 0, 0, 0, 0, 0, 0x1),
                112,
            )
            .unwrap()
        }

        fn pre_allocate_ip_pair(&mut self) -> IpPair {
            self.ip_pool
                .pre_allocate()
                .expect("unexpected ip allocation failure!")
        }

        async fn init_in_memory_storage() -> anyhow::Result<GatewayStorage> {
            let conn_pool = sqlx::sqlite::SqlitePoolOptions::new()
                .connect(":memory:")
                .await
                .context("cannot connect to db")?;
            Ok(GatewayStorage::from_connection_pool(conn_pool, 100).await?)
        }

        async fn mock(rng: &mut impl CryptoRng010) -> anyhow::Result<Self> {
            let base = Party::generate(rng);

            // 1. create in-memory gateway storage
            let storage = Self::init_in_memory_storage().await?;

            // 2. create mock ecash manager for testing (essentially allow **any** credential)
            let ecash_verifier = MockEcashManager::new(Arc::new(storage.clone()));

            let lp_config = LpConfig {
                debug: LpDebug {
                    ..Default::default()
                },
                ..Default::default()
            };
            let forward_semaphore =
                Arc::new(Semaphore::new(lp_config.debug.max_concurrent_forwards));

            // create wireguard data
            let (wireguard_data, peer_request_rx) = Self::wireguard_data(&base);

            let (upgrade_mode_details, _) = mock_dummy_upgrade_mode_details();

            // mock the wg peer controller
            let (mock_peer_controller, peer_controller_state) =
                mock_peer_controller(peer_request_rx);

            // registering particular responses for peer controller is up to given test
            let ecash_verifier = Arc::new(ecash_verifier);

            let peer_registrator = PeerRegistrator::new(
                ecash_verifier.clone(),
                PeerManager::new(wireguard_data),
                upgrade_mode_details,
            );

            let sessions = ActiveLpSessions::new();

            let lp_state = SharedLpControlState {
                local_lp_peer: base.peer.clone(),
                nodes: LpNodes::new_empty(),
                sessions: sessions.clone(),
                metrics: Default::default(),
                lp_config,
            };

            let gateway_lp_state = SharedGatewayLpControlState {
                identity: *base.identity.public_key(),
                sessions,
                registration_nonces: RegistrationNonces::new(),

                forward_semaphore,

                // handles for dealing with new peers
                peer_registrator: Some(peer_registrator),
            };

            Ok(Gateway {
                base,
                lp_state,
                gateway_lp_state,
                ip_pool: Self::ip_pool(),
                mock_peer_controller: SpawnedPeerController::Ready {
                    controller: mock_peer_controller,
                },
                mock_peer_controller_state: peer_controller_state,
                tasks_cancellation: Default::default(),
                lp_connection_handler: SpawnedLpConnectionHandlerState::NotCreated,
            })
        }

        /// Create the handler the listener would for a connection arriving from `remote_addr`.
        fn create_lp_handler(&mut self, connection: MockTransport, remote_addr: SocketAddr) {
            let SpawnedLpConnectionHandlerState::NotCreated = self.lp_connection_handler else {
                panic!("lp connection handler in invalid state")
            };

            // the listener's own lookup: a known node's address takes the node-address path
            let known_node = self.lp_state.nodes.get_node_details(remote_addr.ip());

            self.lp_connection_handler = SpawnedLpConnectionHandlerState::Ready {
                handler: LpIngressConnectionHandler::new(
                    connection,
                    remote_addr,
                    known_node,
                    self.lp_state.clone(),
                    Some(self.gateway_lp_state.clone()),
                ),
            };
        }

        fn advertise_lp_version(&mut self, version: u8) {
            self.base.lp_version = version;
        }

        /// Make this gateway's directory view know `other` as an LP node.
        fn learn_node(&mut self, other: &Party) {
            self.lp_state.nodes.swap_view(HashMap::from([(
                other.socket_addr.ip(),
                other.lp_node_details(),
            )]));
        }

        fn spawn_lp_handler(&mut self) {
            let SpawnedLpConnectionHandlerState::Ready { handler } = mem::replace(
                &mut self.lp_connection_handler,
                SpawnedLpConnectionHandlerState::NotCreated,
            ) else {
                panic!("lp connection handler in invalid state")
            };
            let cancellation_token = self.tasks_cancellation.clone();

            self.lp_connection_handler = SpawnedLpConnectionHandlerState::Running {
                handle: tokio::spawn(async move {
                    let handler_future = async move {
                        handler
                            .handle()
                            .await
                            .inspect_err(|err| error!("lp handler has failed: {err}"))
                    };
                    cancellation_token.run_until_cancelled(handler_future).await
                }),
            }
        }

        fn spawn_peer_controller(&mut self) {
            let SpawnedPeerController::Ready { mut controller } = mem::replace(
                &mut self.mock_peer_controller,
                SpawnedPeerController::Invalid,
            ) else {
                panic!("mock peer controller in invalid state")
            };

            let cancellation_token = self.tasks_cancellation.clone();
            let join_handle = tokio::spawn(async move {
                cancellation_token
                    .run_until_cancelled(controller.run())
                    .await
            });
            self.mock_peer_controller = SpawnedPeerController::Running {
                handle: join_handle,
            }
        }

        #[allow(clippy::panic)]
        async fn stop_tasks(&mut self) -> anyhow::Result<()> {
            self.tasks_cancellation.cancel();

            let SpawnedLpConnectionHandlerState::Running { handle: lp_handle } = mem::replace(
                &mut self.lp_connection_handler,
                SpawnedLpConnectionHandlerState::NotCreated,
            ) else {
                panic!("lp connection handler in invalid state")
            };

            let SpawnedPeerController::Running {
                handle: peer_controller_handle,
            } = mem::replace(
                &mut self.mock_peer_controller,
                SpawnedPeerController::Invalid,
            )
            else {
                panic!("mock peer controller in invalid state")
            };

            lp_handle.timeboxed().await?.context("join failure")?;
            peer_controller_handle
                .timeboxed()
                .await?
                .context("join failure")?;
            self.mock_peer_controller = SpawnedPeerController::Finished;
            self.lp_connection_handler = SpawnedLpConnectionHandlerState::Finished;

            Ok(())
        }
    }

    #[cfg(test)]
    mod using_registration_client {

        //
    }

    // requires additional calls that are automatically included in the top level 'RegistrationClient'
    #[cfg(test)]
    mod using_lp_registration_client {
        use super::*;
        use nym_kkt_ciphersuite::{IntoEnumIterator, KEM};
        use nym_registration_client::NestedLpSession;
        use nym_test_utils::helpers::u64_seeded_rng;
        use nym_wireguard::DefguardPeer;

        #[tokio::test]
        async fn test_basic_lp_entry_registration() -> anyhow::Result<()> {
            // nym_test_utils::helpers::setup_test_logger();

            for kem in KEM::iter() {
                let ciphersuite = Ciphersuite::default().with_kem(kem);

                // initialise random, but deterministic, keys, addresses, etc. for the parties
                let mut client_rng = u64_seeded_rng(0);
                let mut gateway_rng = u64_seeded_rng(1);

                let client_data = Client::mock(&mut client_rng);
                let client_key = *client_data.base.x25519_wg_keys.public_key();
                let mut entry = Gateway::mock(&mut gateway_rng).await?;

                let mut client = LpGatewayClient::<MockTransport>::new_with_default_config(
                    client_data.base.peer.x25519().clone(),
                    entry.base.peer.as_remote(),
                    entry.base.socket_addr,
                    ciphersuite,
                    entry.base.lp_version,
                );

                // 1. establish mock connection between client and gateway and retrieve gateway's handle
                client.ensure_connected().await?;
                let gateway_conn = client
                    .connection()
                    .as_ref()
                    .context("mock connection has failed!")?
                    .try_get_remote_handle();

                // 2. create and spawn gateway handler for the client connection
                entry.create_lp_handler(gateway_conn, client_data.base.socket_addr);
                entry.spawn_lp_handler();

                // 3. register all needed responses for the dvpn registration that will reach the peer controller
                // 1) peer registration - ip pair allocation
                let ip_pair = entry.pre_allocate_ip_pair();
                let reg_res = Ok::<_, nym_wireguard::Error>(ip_pair);

                entry
                    .register_peer_controller_response(
                        PeerControlRequestType::AllocatePeerIpPair {},
                        reg_res,
                    )
                    .await;

                // 2) new peer inclusion - in non-mock system it would spawn handlers,
                // here we'll just set a flag and say it's all fine
                let public_key = client_key.to_wg_key();
                let add_res = Ok::<_, nym_wireguard::Error>(());
                entry
                    .register_peer_controller_response(
                        PeerControlRequestType::AddPeer { public_key },
                        add_res,
                    )
                    .await;

                // 3) peer query - check for prior registrations
                let query_res = Ok::<_, nym_wireguard::Error>(Option::<DefguardPeer>::None);
                let key = client_key.to_wg_key();
                entry
                    .register_peer_controller_response(
                        PeerControlRequestType::QueryPeer { key },
                        query_res,
                    )
                    .await;

                // 4. spawn peer controller to be able to handle dvpn registration requests
                entry.spawn_peer_controller();

                // 5. perform client handshake
                client.perform_handshake().timeboxed().await??;

                // 6. perform registration with entry only
                let wg_keypair = client_data.base.x25519_wg_keys;
                let gateway_identity = entry.base.identity.public_key();
                let registration_result = LpDvpnRegistrationClient::new(&mut client)
                    .register(
                        &mut client_rng,
                        &wg_keypair,
                        gateway_identity,
                        &client_data.ticket_provider,
                        None,
                        TicketType::V1WireguardEntry,
                    )
                    .timeboxed()
                    .await??;

                // 7. verify registration result
                let peers_guard = entry.mock_peer_controller_state.peers.read().await;
                let peer = peers_guard.get_by_x25519_key(&client_key).unwrap().clone();
                drop(peers_guard);
                assert!(peer.add_success);

                assert_eq!(registration_result.private_ipv4, ip_pair.ipv4);
                assert_eq!(registration_result.private_ipv6, ip_pair.ipv6);
                assert_eq!(
                    registration_result.public_key,
                    *entry.base.x25519_wg_keys.public_key()
                );

                // 8. stop the gateway task and finish the test
                entry.stop_tasks().await?;
            }

            Ok(())
        }

        #[tokio::test]
        async fn registration_is_not_allowed_without_prior_handshake() -> anyhow::Result<()> {
            // nym_test_utils::helpers::setup_test_logger();
            // initialise random, but deterministic, keys, addresses, etc. for the parties
            let mut client_rng = u64_seeded_rng(0);
            let mut gateway_rng = u64_seeded_rng(1);

            let client_data = Client::mock(&mut client_rng);
            let mut entry = Gateway::mock(&mut gateway_rng).await?;

            let ciphersuite = Ciphersuite::default();

            let mut client = LpGatewayClient::<MockTransport>::new_with_default_config(
                client_data.base.peer.x25519().clone(),
                entry.base.peer.as_remote(),
                entry.base.socket_addr,
                ciphersuite,
                entry.base.lp_version,
            );

            // 1. establish mock connection between client and gateway and retrieve gateway's handle
            client.ensure_connected().await?;
            let gateway_conn = client
                .connection()
                .as_ref()
                .context("mock connection has failed!")?
                .try_get_remote_handle();

            // 2. create and spawn gateway handler for the client connection
            entry.create_lp_handler(gateway_conn, client_data.base.socket_addr);
            entry.spawn_lp_handler();

            // 3. spawn peer controller to be able to handle dvpn registration requests
            // (which we shouldn't receive anyway)
            entry.spawn_peer_controller();

            // 4. perform registration with entry only
            // but WITHOUT performing the handshake
            let wg_keypair = client_data.base.x25519_wg_keys;
            let gateway_identity = entry.base.identity.public_key();
            let registration_result = LpDvpnRegistrationClient::new(&mut client)
                .register(
                    &mut client_rng,
                    &wg_keypair,
                    gateway_identity,
                    &client_data.ticket_provider,
                    None,
                    TicketType::V1WireguardEntry,
                )
                .timeboxed()
                .await?
                .unwrap_err();

            let LpClientError::IncompleteHandshake = registration_result else {
                panic!("unexpected error");
            };

            // 5. stop the gateway task and finish the test
            entry.stop_tasks().await?;
            Ok(())
        }

        async fn run_telescoped_registration(
            gateway_lp_version: u8,
            exit_knows_entry: bool,
        ) -> anyhow::Result<()> {
            // nym_test_utils::helpers::setup_test_logger();

            // TODO: update the test once mceliece works
            let kem = KEM::MlKem768;

            let ciphersuite = Ciphersuite::default().with_kem(kem);

            // initialise random, but deterministic, keys, addresses, etc. for the parties
            let mut client_rng = u64_seeded_rng(0);
            let mut entry_rng = u64_seeded_rng(1);
            let mut exit_rng = u64_seeded_rng(2);

            let client_data = Client::mock(&mut client_rng);
            let client_key = *client_data.base.x25519_wg_keys.public_key();
            let mut entry = Gateway::mock(&mut entry_rng).await?;
            let mut exit = Gateway::mock(&mut exit_rng).await?;

            entry.advertise_lp_version(gateway_lp_version);
            exit.advertise_lp_version(gateway_lp_version);
            if exit_knows_entry {
                exit.learn_node(&entry.base);
            }

            let mut entry_client = LpGatewayClient::<MockTransport>::new_with_default_config(
                client_data.base.peer.x25519().clone(),
                entry.base.peer.as_remote(),
                entry.base.socket_addr,
                ciphersuite,
                entry.base.lp_version,
            );

            // START: ENTRY SETUP
            //
            // 1. establish mock connection between client and gateway and retrieve gateway's handle
            entry_client.ensure_connected().await?;
            let entry_conn = entry_client
                .connection()
                .as_ref()
                .context("mock connection has failed!")?
                .try_get_remote_handle();
            entry_conn.set_id(1);

            // 2. the exit's end of the connection the entry opens when it first forwards
            let exit_conn = MockTransport::expect_connection(exit.base.socket_addr);
            exit_conn.set_id(255);

            // 3. create handler for the client connection (entry)
            entry.create_lp_handler(entry_conn, client_data.base.socket_addr);

            // 4. register all needed responses for the dvpn registration that will reach the peer controller
            // 1) peer registration - ip pair allocation
            let entry_ip_pair = entry.pre_allocate_ip_pair();
            let reg_res = Ok::<_, nym_wireguard::Error>(entry_ip_pair);

            entry
                .register_peer_controller_response(
                    PeerControlRequestType::AllocatePeerIpPair {},
                    reg_res,
                )
                .await;

            // 2) new peer inclusion - in non-mock system it would spawn handlers,
            // here we'll just set a flag and say it's all fine
            let public_key = client_key.to_wg_key();
            let add_res = Ok::<_, nym_wireguard::Error>(());
            entry
                .register_peer_controller_response(
                    PeerControlRequestType::AddPeer { public_key },
                    add_res,
                )
                .await;

            // 3) peer query - check for prior registrations
            let query_res = Ok::<_, nym_wireguard::Error>(Option::<DefguardPeer>::None);
            let key = client_key.to_wg_key();
            entry
                .register_peer_controller_response(
                    PeerControlRequestType::QueryPeer { key },
                    query_res,
                )
                .await;

            // 5. spawn peer controller to be able to handle dvpn registration requests
            entry.spawn_peer_controller();

            // 6. finally spawn the handler
            entry.spawn_lp_handler();

            // 7. perform client handshake (with the entry)
            entry_client.perform_handshake().timeboxed().await??;

            // END: ENTRY SETUP
            //
            // START: EXIT SETUP:
            // 8. create handler for the forwarding channel (exit); it arrives from the entry's address
            exit.create_lp_handler(exit_conn, entry.base.socket_addr);

            // 9. spawn the handler
            exit.spawn_lp_handler();

            // 10. register all needed responses for the dvpn registration that will reach the peer controller
            // 1) peer registration - ip pair allocation
            let exit_ip_pair = exit.pre_allocate_ip_pair();
            let reg_res = Ok::<_, nym_wireguard::Error>(exit_ip_pair);

            exit.register_peer_controller_response(
                PeerControlRequestType::AllocatePeerIpPair {},
                reg_res,
            )
            .await;

            // 2) new peer inclusion - in non-mock system it would spawn handlers,
            // here we'll just set a flag and say it's all fine
            let public_key = client_key.to_wg_key();
            let add_res = Ok::<_, nym_wireguard::Error>(());
            exit.register_peer_controller_response(
                PeerControlRequestType::AddPeer { public_key },
                add_res,
            )
            .await;

            // 3) peer query - check for prior registrations
            let query_res = Ok::<_, nym_wireguard::Error>(Option::<DefguardPeer>::None);
            let key = client_key.to_wg_key();
            exit.register_peer_controller_response(
                PeerControlRequestType::QueryPeer { key },
                query_res,
            )
            .await;

            // 11. spawn peer controller to be able to handle dvpn registration requests
            exit.spawn_peer_controller();

            // END: EXIT SETUP

            // 12. create nested session to register with exit via forwarding
            // technically we should use different ephemeral keys than we had for the entry
            // but crypto is going to work the same
            let mut nested_session = NestedLpSession::new(
                exit.base.socket_addr,
                client_data.base.peer.x25519().clone(),
                exit.base.peer.as_remote(),
                ciphersuite,
                exit.base.lp_version,
            );

            // 13. Perform handshake and registration with exit gateway (all via entry forwarding)
            nested_session
                .perform_handshake(&mut entry_client)
                .timeboxed()
                .await??;

            let exit_registration_result =
                NestedLpDvpnRegistrationClient::new(&mut nested_session, &mut entry_client)
                    .register(
                        &mut client_rng,
                        &client_data.base.x25519_wg_keys,
                        exit.base.identity.public_key(),
                        &client_data.ticket_provider,
                        None,
                        TicketType::V1WireguardExit,
                    )
                    .timeboxed()
                    .await??;

            // 14. complete registration with the entry
            let entry_registration_result = LpDvpnRegistrationClient::new(&mut entry_client)
                .register(
                    &mut client_rng,
                    &client_data.base.x25519_wg_keys,
                    entry.base.identity.public_key(),
                    &client_data.ticket_provider,
                    None,
                    TicketType::V1WireguardEntry,
                )
                .timeboxed()
                .await??;

            // 15. verify all registration results
            let peers_guard = entry.mock_peer_controller_state.peers.read().await;
            let entry_peer = peers_guard.get_by_x25519_key(&client_key).unwrap().clone();
            drop(peers_guard);
            assert!(entry_peer.add_success);

            let peers_guard = exit.mock_peer_controller_state.peers.read().await;
            let exit_peer = peers_guard.get_by_x25519_key(&client_key).unwrap().clone();
            drop(peers_guard);
            assert!(exit_peer.add_success);

            assert_eq!(entry_registration_result.private_ipv4, entry_ip_pair.ipv4);
            assert_eq!(entry_registration_result.private_ipv6, entry_ip_pair.ipv6);
            assert_eq!(
                entry_registration_result.public_key,
                *entry.base.x25519_wg_keys.public_key()
            );

            assert_eq!(exit_registration_result.private_ipv4, exit_ip_pair.ipv4);
            assert_eq!(exit_registration_result.private_ipv6, exit_ip_pair.ipv6);
            assert_eq!(
                exit_registration_result.public_key,
                *exit.base.x25519_wg_keys.public_key()
            );

            // 16. stop the gateway task and finish the test
            entry.stop_tasks().await?;
            exit.stop_tasks().await?;

            Ok(())
        }

        #[tokio::test]
        async fn test_basic_lp_exit_registration() -> anyhow::Result<()> {
            run_telescoped_registration(version::CURRENT, false).await
        }

        /// The exit serves the entry's forwarded connection the way its listener would for a known node IP.
        #[tokio::test]
        async fn telescoped_registration_when_the_exit_knows_the_entry_as_a_node()
        -> anyhow::Result<()> {
            run_telescoped_registration(version::CURRENT, true).await
        }

        /// Gateways still advertising version 1 make the client send version 1 openers.
        #[tokio::test]
        async fn telescoped_registration_with_gateways_advertising_v1() -> anyhow::Result<()> {
            run_telescoped_registration(version::V1, true).await
        }
    }
}
