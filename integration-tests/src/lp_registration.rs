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
    use nym_node::config::{LpConfig, LpDebug};
    use nym_node::node::GatewayStorage;
    use nym_node::node::lp::control::handler::LpConnectionHandler;
    use nym_node::node::lp::error::LpHandlerError;
    use nym_node::node::lp::{SharedLpControlState, SharedLpState};
    use nym_node::wireguard::{PeerManager, PeerRegistrator};
    use nym_registration_client::{LpClientError, LpRegistrationClient};
    use nym_test_utils::helpers::{CryptoRng09, seeded_rng};
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::Timeboxed;
    use nym_wireguard::peer_controller::IpPair;
    use nym_wireguard::peer_controller::mock::{
        Key, KeyWrapper, MockPeerController, MockPeerControllerState, PeerControlRequestType,
        RegisteredResponse, mock_peer_controller,
    };
    use nym_wireguard::{IpPool, PeerControlRequest, WireguardConfig, WireguardGatewayData};
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
        fn generate(rng: &mut impl CryptoRng09) -> Self {
            let mut ip = [0u8; 4];
            let mut port = [0u8; 2];

            // generate a valid instance of rand08
            let mut seed = [0u8; 32];
            rng.fill_bytes(&mut seed);
            let mut rng08 = seeded_rng(seed);

            rng.fill_bytes(&mut ip);
            rng.fill_bytes(&mut port);
            let ed25519_keys = ed25519::KeyPair::new(&mut rng08);
            let x25519_wg_keys = Arc::new(x25519::KeyPair::new(&mut rng08));

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
                lp_version: 1,
            }
        }
    }

    struct Client {
        base: Party,

        ticket_provider: MockBandwidthController,
    }

    impl Client {
        fn mock(rng: &mut impl CryptoRng09) -> Self {
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
            handler: LpConnectionHandler<MockIOStream>,
        },
        Running {
            handle: JoinHandle<Option<Result<(), LpHandlerError>>>,
        },
        Finished,
    }

    struct Gateway {
        base: Party,
        lp_state: SharedLpControlState,
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

        async fn mock(rng: &mut impl CryptoRng09) -> anyhow::Result<Self> {
            let base = Party::generate(rng);

            // 1. create in-memory gateway storage
            let storage = Self::init_in_memory_storage().await?;

            // 2. create mock ecash manager for testing (essentially allow **any** credential)
            let ecash_verifier = MockEcashManager::new(Box::new(storage.clone()));

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

            let lp_state = SharedLpControlState {
                local_lp_peer: base.peer.clone(),

                forward_semaphore,

                // handles for dealing with new peers
                peer_registrator: Some(peer_registrator),
                shared: SharedLpState {
                    metrics: Default::default(),
                    lp_config,
                    session_states: Arc::new(Default::default()),
                },
            };

            Ok(Gateway {
                base,
                lp_state,
                ip_pool: Self::ip_pool(),
                mock_peer_controller: SpawnedPeerController::Ready {
                    controller: mock_peer_controller,
                },
                mock_peer_controller_state: peer_controller_state,
                tasks_cancellation: Default::default(),
                lp_connection_handler: SpawnedLpConnectionHandlerState::NotCreated,
            })
        }

        fn create_lp_handler(
            &mut self,
            client_connection: MockIOStream,
            client_address: SocketAddr,
        ) {
            let SpawnedLpConnectionHandlerState::NotCreated = self.lp_connection_handler else {
                panic!("lp connection handler in invalid state")
            };

            self.lp_connection_handler = SpawnedLpConnectionHandlerState::Ready {
                handler: LpConnectionHandler::new(
                    client_connection,
                    client_address,
                    self.lp_state.clone(),
                ),
            };
        }

        async fn establish_forwarding_channel(
            &mut self,
            exit_address: SocketAddr,
        ) -> anyhow::Result<MockIOStream> {
            let SpawnedLpConnectionHandlerState::Ready { handler } =
                &mut self.lp_connection_handler
            else {
                panic!("lp connection handler in invalid state")
            };

            handler.establish_exit_stream(exit_address).await?;
            Ok(handler
                .forwarding_channel()
                .as_ref()
                .context("mock connection has failed!")?
                .0
                .try_get_remote_handle())
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
        use nym_test_utils::helpers::u64_seeded_rng_09;
        use nym_wireguard::DefguardPeer;

        #[tokio::test]
        async fn test_basic_lp_entry_registration() -> anyhow::Result<()> {
            // nym_test_utils::helpers::setup_test_logger();

            for kem in KEM::iter() {
                let ciphersuite = Ciphersuite::default().with_kem(kem);

                // initialise random, but deterministic, keys, addresses, etc. for the parties
                let mut client_rng = u64_seeded_rng_09(0);
                let mut gateway_rng = u64_seeded_rng_09(1);

                let client_data = Client::mock(&mut client_rng);
                let client_key = *client_data.base.x25519_wg_keys.public_key();
                let mut entry = Gateway::mock(&mut gateway_rng).await?;

                let mut client = LpRegistrationClient::<MockIOStream>::new_with_default_config(
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
                let registration_result = client
                    .register_dvpn(
                        &mut client_rng,
                        &wg_keypair,
                        gateway_identity,
                        &client_data.ticket_provider,
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
            let mut client_rng = u64_seeded_rng_09(0);
            let mut gateway_rng = u64_seeded_rng_09(1);

            let client_data = Client::mock(&mut client_rng);
            let mut entry = Gateway::mock(&mut gateway_rng).await?;

            let ciphersuite = Ciphersuite::default();

            let mut client = LpRegistrationClient::<MockIOStream>::new_with_default_config(
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
            let registration_result = client
                .register_dvpn(
                    &mut client_rng,
                    &wg_keypair,
                    gateway_identity,
                    &client_data.ticket_provider,
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

        #[tokio::test]
        async fn test_basic_lp_exit_registration() -> anyhow::Result<()> {
            // nym_test_utils::helpers::setup_test_logger();

            // TODO: update the test once mceliece works
            let kem = KEM::MlKem768;

            let ciphersuite = Ciphersuite::default().with_kem(kem);

            // initialise random, but deterministic, keys, addresses, etc. for the parties
            let mut client_rng = u64_seeded_rng_09(0);
            let mut entry_rng = u64_seeded_rng_09(1);
            let mut exit_rng = u64_seeded_rng_09(2);

            let client_data = Client::mock(&mut client_rng);
            let client_key = *client_data.base.x25519_wg_keys.public_key();
            let mut entry = Gateway::mock(&mut entry_rng).await?;
            let mut exit = Gateway::mock(&mut exit_rng).await?;

            let mut entry_client = LpRegistrationClient::<MockIOStream>::new_with_default_config(
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

            // 2. create handler for the client connection (entry)
            entry.create_lp_handler(entry_conn, client_data.base.socket_addr);

            // 3. pre-establish connection between entry and exit
            let exit_conn = entry
                .establish_forwarding_channel(exit.base.socket_addr)
                .await?;
            exit_conn.set_id(255);

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
            // 8. create handler for the forwarding channel (exit)
            exit.create_lp_handler(exit_conn, client_data.base.socket_addr);

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
            nested_session.perform_handshake(&mut entry_client).await?;

            let exit_registration_result = nested_session
                .register_dvpn(
                    &mut entry_client,
                    &mut client_rng,
                    &client_data.base.x25519_wg_keys,
                    exit.base.identity.public_key(),
                    &client_data.ticket_provider,
                    TicketType::V1WireguardExit,
                )
                .timeboxed()
                .await??;

            // 14. complete registration with the entry
            let entry_registration_result = entry_client
                .register_dvpn(
                    &mut client_rng,
                    &client_data.base.x25519_wg_keys,
                    entry.base.identity.public_key(),
                    &client_data.ticket_provider,
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
    }
}
