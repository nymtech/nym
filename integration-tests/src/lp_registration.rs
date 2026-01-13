// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use nym_bandwidth_controller::mock::MockBandwidthController;
    use nym_credential_verification::ecash::MockEcashManager;
    use nym_credentials_interface::TicketType;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_gateway::GatewayError;
    use nym_gateway::node::lp_listener::handler::LpConnectionHandler;
    use nym_gateway::node::lp_listener::{
        LpDebug, LpHandlerState, MixForwardingReceiver, PeerControlRequest, WireguardGatewayData,
        mix_forwarding_channels,
    };
    use nym_gateway::node::{ActiveClientsStore, GatewayStorage, LpConfig};
    use nym_registration_client::{LpClientError, LpRegistrationClient};
    use nym_test_utils::helpers::{CryptoRng, RngCore, u64_seeded_rng};
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::Timeboxed;
    use nym_wireguard::peer_controller::IpPair;
    use nym_wireguard::peer_controller::mock::{
        Key, KeyWrapper, MockPeerController, MockPeerControllerState, PeerControlRequestType,
        RegisteredResponse, mock_peer_controller,
    };
    use nym_wireguard::{IpPool, WireguardConfig};
    use std::mem;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::sync::Arc;
    use std::time::Duration;
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
        ed25519_keys: Arc<ed25519::KeyPair>,
        x25519_wg_keys: Arc<x25519::KeyPair>,
        socket_addr: SocketAddr,
    }

    impl Party {
        fn generate(rng: &mut (impl RngCore + CryptoRng)) -> Self {
            let mut ip = [0u8; 4];
            let mut port = [0u8; 2];

            rng.fill_bytes(&mut ip);
            rng.fill_bytes(&mut port);

            Party {
                ed25519_keys: Arc::new(ed25519::KeyPair::new(rng)),
                x25519_wg_keys: Arc::new(x25519::KeyPair::new(rng)),
                socket_addr: SocketAddr::from((ip, u16::from_le_bytes(port))),
            }
        }
    }

    struct Client {
        base: Party,

        ticket_provider: MockBandwidthController,
    }

    impl Client {
        fn mock(rng: &mut (impl RngCore + CryptoRng)) -> Self {
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
            handle: JoinHandle<Option<Result<(), GatewayError>>>,
        },
        Finished,
    }

    struct Gateway {
        base: Party,
        lp_state: LpHandlerState,
        ip_pool: IpPool,
        // might be used later for mixnet registration tests
        #[allow(unused)]
        mix_receiver: MixForwardingReceiver,
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

        async fn allocate_ip_pair(&mut self) -> IpPair {
            self.ip_pool
                .allocate()
                .await
                .expect("unexpected ip allocation failure!")
        }

        async fn init_in_memory_storage() -> anyhow::Result<GatewayStorage> {
            let conn_pool = sqlx::sqlite::SqlitePoolOptions::new()
                .connect(":memory:")
                .await
                .context("cannot connect to db")?;
            Ok(GatewayStorage::from_connection_pool(conn_pool, 100).await?)
        }

        async fn mock(rng: &mut (impl RngCore + CryptoRng)) -> anyhow::Result<Self> {
            let base = Party::generate(rng);

            // 1. create in-memory gateway storage
            let storage = Self::init_in_memory_storage().await?;

            // 2. create mock ecash manager for testing (essentially allow **any** credential)
            let ecash_verifier = MockEcashManager::new(Box::new(storage.clone()));

            let lp_config = LpConfig {
                debug: LpDebug {
                    timestamp_tolerance: Duration::from_secs(30),
                    ..Default::default()
                },
                ..Default::default()
            };
            let forward_semaphore =
                Arc::new(Semaphore::new(lp_config.debug.max_concurrent_forwards));

            // Create mix forwarding channel (unused in tests but required by struct)
            let (mix_sender, mix_receiver) = mix_forwarding_channels();

            // create wireguard data
            let (wireguard_data, peer_request_rx) = Self::wireguard_data(&base);

            // mock the wg peer controller
            let (mock_peer_controller, peer_controller_state) =
                mock_peer_controller(peer_request_rx);

            // registering particular responses for peer controller is up to given test

            let lp_state = LpHandlerState {
                // use mock instance of ecash verifier
                ecash_verifier: Arc::new(ecash_verifier),

                // use in-memory database (no need for persistency)
                storage,

                // reuse the same identity we just generated
                local_identity: base.ed25519_keys.clone(),

                // we don't care about metrics - all zeroes are perfectly fine
                metrics: Default::default(),

                // no clients at the beginning
                active_clients_store: ActiveClientsStore::new(),

                // handles required for wg registration
                wg_peer_controller: Some(wireguard_data.peer_tx().clone()),

                wireguard_data: Some(wireguard_data),

                // use default lp config (with enabled flag)
                lp_config,

                // TODO: might be needed later on for mixnet registration
                outbound_mix_sender: mix_sender,

                // we start with empty state
                handshake_states: Arc::new(Default::default()),

                // we start with empty state
                session_states: Arc::new(Default::default()),

                // sensible default value for tests
                forward_semaphore,
            };

            Ok(Gateway {
                base,
                lp_state,
                ip_pool: Self::ip_pool(),
                mix_receiver,
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
        use nym_registration_client::NestedLpSession;

        #[tokio::test]
        async fn test_basic_lp_entry_registration() -> anyhow::Result<()> {
            // nym_test_utils::helpers::setup_test_logger();
            // initialise random, but deterministic, keys, addresses, etc. for the parties
            let mut client_rng = u64_seeded_rng(0);
            let mut gateway_rng = u64_seeded_rng(1);

            let client_data = Client::mock(&mut client_rng);
            let client_key = *client_data.base.x25519_wg_keys.public_key();
            let mut entry = Gateway::mock(&mut gateway_rng).await?;

            let mut client = LpRegistrationClient::<MockIOStream>::new_with_default_psk(
                client_data.base.ed25519_keys,
                *entry.base.ed25519_keys.public_key(),
                entry.base.socket_addr,
                client_data.base.socket_addr.ip(),
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
            let ip_pair = entry.allocate_ip_pair().await;
            let reg_res = Ok::<_, nym_wireguard::Error>(ip_pair);
            let public_key = client_key.to_wg_key();

            entry
                .register_peer_controller_response(
                    PeerControlRequestType::RegisterPeer { public_key },
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

            // 4. spawn peer controller to be able to handle dvpn registration requests
            entry.spawn_peer_controller();

            // 5. perform client handshake
            client.perform_handshake().timeboxed().await??;

            // 6. perform registration with entry only
            let wg_keypair = client_data.base.x25519_wg_keys;
            let gateway_identity = entry.base.ed25519_keys.public_key();
            let registration_result = client
                .register(
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
            assert!(peer.register_success);
            assert!(peer.add_success);

            assert_eq!(registration_result.private_ipv4, ip_pair.ipv4);
            assert_eq!(registration_result.private_ipv6, ip_pair.ipv6);
            assert_eq!(
                registration_result.public_key,
                *entry.base.x25519_wg_keys.public_key()
            );

            // 8. stop the gateway task and finish the test
            entry.stop_tasks().await?;
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

            let mut client = LpRegistrationClient::<MockIOStream>::new_with_default_psk(
                client_data.base.ed25519_keys,
                *entry.base.ed25519_keys.public_key(),
                entry.base.socket_addr,
                client_data.base.socket_addr.ip(),
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
            let gateway_identity = entry.base.ed25519_keys.public_key();
            let registration_result = client
                .register(
                    &wg_keypair,
                    gateway_identity,
                    &client_data.ticket_provider,
                    TicketType::V1WireguardEntry,
                )
                .timeboxed()
                .await?
                .unwrap_err();

            let LpClientError::Transport(err) = registration_result else {
                panic!("unexpected error");
            };
            assert_eq!(err, "Cannot register: handshake not completed");

            // 5. stop the gateway task and finish the test
            entry.stop_tasks().await?;
            Ok(())
        }

        #[tokio::test]
        async fn test_basic_lp_exit_registration() -> anyhow::Result<()> {
            // initialise random, but deterministic, keys, addresses, etc. for the parties
            let mut client_rng = u64_seeded_rng(0);
            let mut entry_rng = u64_seeded_rng(1);
            let mut exit_rng = u64_seeded_rng(2);

            let client_data = Client::mock(&mut client_rng);
            let client_key = *client_data.base.x25519_wg_keys.public_key();
            let mut entry = Gateway::mock(&mut entry_rng).await?;
            let mut exit = Gateway::mock(&mut exit_rng).await?;

            let mut entry_client = LpRegistrationClient::<MockIOStream>::new_with_default_psk(
                client_data.base.ed25519_keys.clone(),
                *entry.base.ed25519_keys.public_key(),
                entry.base.socket_addr,
                client_data.base.socket_addr.ip(),
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
            let entry_ip_pair = entry.allocate_ip_pair().await;
            let reg_res = Ok::<_, nym_wireguard::Error>(entry_ip_pair);
            let public_key = client_key.to_wg_key();

            entry
                .register_peer_controller_response(
                    PeerControlRequestType::RegisterPeer { public_key },
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
            let exit_ip_pair = exit.allocate_ip_pair().await;
            let reg_res = Ok::<_, nym_wireguard::Error>(exit_ip_pair);
            let public_key = client_key.to_wg_key();

            exit.register_peer_controller_response(
                PeerControlRequestType::RegisterPeer { public_key },
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

            // 11. spawn peer controller to be able to handle dvpn registration requests
            exit.spawn_peer_controller();

            // END: EXIT SETUP

            // 12. create nested session to register with exit via forwarding
            // technically we should use different ephemeral keys than we had for the entry
            // but crypto is going to work the same
            let mut nested_session = NestedLpSession::new(
                exit.base.ed25519_keys.public_key().to_bytes(),
                exit.base.socket_addr.to_string(),
                client_data.base.ed25519_keys,
                *exit.base.ed25519_keys.public_key(),
            );

            // 13. Perform handshake and registration with exit gateway (all via entry forwarding)
            let exit_registration_result = nested_session
                .handshake_and_register(
                    &mut entry_client,
                    &client_data.base.x25519_wg_keys,
                    exit.base.ed25519_keys.public_key(),
                    &client_data.ticket_provider,
                    TicketType::V1WireguardExit,
                    client_data.base.socket_addr.ip(),
                )
                .timeboxed()
                .await??;

            // 14. complete registration with the entry
            let entry_registration_result = entry_client
                .register(
                    &client_data.base.x25519_wg_keys,
                    entry.base.ed25519_keys.public_key(),
                    &client_data.ticket_provider,
                    TicketType::V1WireguardEntry,
                )
                .timeboxed()
                .await??;

            // 15. verify all registration results
            let peers_guard = entry.mock_peer_controller_state.peers.read().await;
            let entry_peer = peers_guard.get_by_x25519_key(&client_key).unwrap().clone();
            drop(peers_guard);
            assert!(entry_peer.register_success);
            assert!(entry_peer.add_success);

            let peers_guard = exit.mock_peer_controller_state.peers.read().await;
            let exit_peer = peers_guard.get_by_x25519_key(&client_key).unwrap().clone();
            drop(peers_guard);
            assert!(exit_peer.register_success);
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
