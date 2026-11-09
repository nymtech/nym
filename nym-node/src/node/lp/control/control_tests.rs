// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use crate::node::lp::active_sessions::{ActiveLpSessions, LpPeer};
    use crate::node::lp::control::egress::connection::InitialLpEgressNodeConnectionHandler;
    use crate::node::lp::control::ingress::connection_handler::LpIngressConnectionHandler;
    use crate::node::lp::directory::LpNodeDetails;
    use crate::node::lp::directory::LpNodes;
    use crate::node::lp::error::LpHandlerError;
    use crate::node::lp::state::{SharedGatewayLpControlState, SharedLpControlState};
    use nym_crypto::asymmetric::ed25519;
    use nym_lp::peer::{
        KEMKeys, LpLocalPeer, LpRemotePeer, generate_keypair_mceliece, generate_keypair_mlkem,
        generate_lp_keypair_x25519, mock_peers,
    };
    use nym_lp::psq::initiator::HandshakeMode;
    use nym_lp::transport::mock::MockTransport;
    use nym_lp::{Ciphersuite, LpTransportSession};
    use nym_lp_data::packet::version;
    use nym_test_utils::helpers::{deterministic_rng, seeded_rng};
    use nym_test_utils::traits::{Timeboxed, TimeboxedSpawnable};
    use rand::Rng;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use tokio::io::AsyncReadExt;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::task::JoinHandle;
    use tokio::time::error::Elapsed;

    fn shared_node_state(peer: LpLocalPeer) -> SharedLpControlState {
        SharedLpControlState {
            local_lp_peer: peer,
            nodes: LpNodes::new_empty(),
            sessions: Default::default(),
            metrics: Default::default(),
            lp_config: Default::default(),
        }
    }

    fn gateway_state(state: &SharedLpControlState) -> SharedGatewayLpControlState {
        let identity = ed25519::KeyPair::new(&mut deterministic_rng());
        SharedGatewayLpControlState::new(
            *identity.public_key(),
            state.sessions.clone(),
            None,
            &state.lp_config,
        )
    }

    fn lp_node_details(peer: LpRemotePeer) -> LpNodeDetails {
        let key_bytes = peer.x25519().as_ref().try_into().unwrap();
        let mut rng = seeded_rng(key_bytes);
        LpNodeDetails::new(
            rng.next_u32(),
            peer.kem_key_digests().clone(),
            *peer.x25519(),
            1234,
            1235,
            version::CURRENT,
        )
    }

    #[tokio::test]
    async fn basic_node_to_node_handshake() -> anyhow::Result<()> {
        // nym_test_utils::helpers::setup_test_logger();

        let (init, resp) = mock_peers();
        let init_remote = init.as_remote();
        let resp_remote = resp.as_remote();

        let conn_init = MockTransport::default();
        let conn_resp = conn_init.try_get_remote_handle();

        let init_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234);
        let init_details = lp_node_details(init_remote);
        let resp_details = lp_node_details(resp_remote);

        let init_state = shared_node_state(init);
        let resp_state = shared_node_state(resp);

        // keep handles so we can assert on what ended up stored
        let init_sessions = init_state.sessions.clone();
        let resp_sessions = resp_state.sessions.clone();

        let init_handler = InitialLpEgressNodeConnectionHandler::new(
            conn_init,
            init_addr,
            resp_details,
            init_state,
        );

        let resp_handler = LpIngressConnectionHandler::new(
            conn_resp,
            init_addr,
            Some(init_details),
            resp_state,
            None,
        );

        let init_future = init_handler.complete_initial_handshake().spawn_timeboxed();
        let resp_future = resp_handler.handle().spawn_timeboxed();

        let (init_result, resp_result) = tokio::join!(init_future, resp_future);
        let init_index = init_result???;
        resp_result???;

        // the egress side stored its session against the peer, so the data plane can find it
        assert_eq!(
            init_sessions.sending_index_for(LpPeer::node(init_addr.ip())),
            Some(init_index)
        );

        // the ingress side stored its session against the peer, so the data plane can find it
        assert_eq!(
            resp_sessions.sending_index_for(LpPeer::node(init_addr.ip())),
            Some(init_index)
        );

        Ok(())
    }

    /// A second session for the same peer becomes the sending one and demotes the first,
    /// which stays resolvable so in-flight packets still decrypt.
    #[tokio::test]
    async fn second_session_demotes_the_first() -> anyhow::Result<()> {
        use nym_lp::SessionsMock;
        use nym_lp_data::packet::LpFrame;
        use nym_lp_data::packet::frame::LpFrameKind;

        let peer_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let sessions = ActiveLpSessions::new();

        let first = SessionsMock::mock_seeded_post_handshake(1, nym_lp::KEM::MlKem768).initiator;
        let second = SessionsMock::mock_seeded_post_handshake(2, nym_lp::KEM::MlKem768).initiator;
        let first_index = first.receiver_index();
        let second_index = second.receiver_index();
        assert_ne!(first_index, second_index);

        sessions.insert_addressed_session(LpPeer::node(peer_ip), first)?;
        assert_eq!(
            sessions.sending_index_for(LpPeer::node(peer_ip)),
            Some(first_index)
        );

        sessions.insert_addressed_session(LpPeer::node(peer_ip), second)?;

        // the newest session is the one used for sending
        assert_eq!(
            sessions.sending_index_for(LpPeer::node(peer_ip)),
            Some(second_index)
        );

        // the first is demoted: still present, but refuses to encrypt
        let demoted = sessions.with_session_mut(first_index, |s| s.is_read_only())?;
        assert!(demoted);

        let frame = LpFrame::new(LpFrameKind::Opaque, b"nope".to_vec());
        let send_on_demoted = sessions.with_session_mut(first_index, |s| {
            s.process_input(nym_lp::session::LpInput::SendFrame(frame))
        })?;
        assert!(matches!(
            send_on_demoted,
            Err(nym_lp::LpError::SessionReadOnly { .. })
        ));

        // ... while the promoted one still sends fine
        let frame = LpFrame::new(LpFrameKind::Opaque, b"yes".to_vec());
        assert!(sessions.send_frame(LpPeer::node(peer_ip), frame).is_ok());

        Ok(())
    }

    /// One client as the node sees it: its ingress handler running on a real socket, and the
    /// client's side of that socket once the one-way handshake has completed.
    struct ServedClient {
        handler: JoinHandle<Result<Result<(), LpHandlerError>, Elapsed>>,
        stream: TcpStream,
        session: LpTransportSession,
        remote_addr: SocketAddr,
    }

    /// Accepts one connection as the listener would, taking `known_node` for the connecting
    /// address, and completes `client_peer`'s one-way handshake over it.
    async fn serve_one_way_opener(
        state: SharedLpControlState,
        gateway_state: Option<SharedGatewayLpControlState>,
        client_peer: LpLocalPeer,
        known_node: Option<LpNodeDetails>,
    ) -> anyhow::Result<ServedClient> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let mut stream = TcpStream::connect(listener.local_addr()?).await?;
        let (accepted, remote_addr) = listener.accept().await?;

        let exit_remote = state.local_lp_peer.as_remote();

        let handler = LpIngressConnectionHandler::new(
            accepted,
            remote_addr,
            known_node,
            state,
            gateway_state,
        )
        .handle()
        .spawn_timeboxed();

        let session = LpTransportSession::psq_handshake_initiator(
            &mut stream,
            client_peer,
            exit_remote,
            version::CURRENT,
            HandshakeMode::OneWayExit,
        )?
        .complete_handshake()
        .timeboxed()
        .await??;

        Ok(ServedClient {
            handler,
            stream,
            session,
            remote_addr,
        })
    }

    /// What a forwarded client looks like from the exit: a one-way opener arriving from a node's IP.
    /// The digests the exit holds for that IP are not the client's key, and a one-way opener never
    /// claims to be that node anyway.
    #[tokio::test]
    async fn a_one_way_opener_from_a_known_node_is_served_as_a_client() -> anyhow::Result<()> {
        let (client_peer, exit) = mock_peers();
        let state = shared_node_state(exit);
        let gateway = gateway_state(&state);
        let known_node = Some(lp_node_details(client_peer.as_remote()));

        let client =
            serve_one_way_opener(state.clone(), Some(gateway), client_peer, known_node).await?;

        // closing the client's side is what ends the handler's frame loop
        drop(client.stream);
        client.handler.await???;

        assert_eq!(
            state
                .sessions
                .sending_index_for(LpPeer::node(client.remote_addr.ip())),
            None
        );
        assert!(
            state
                .sessions
                .with_session_mut(client.session.receiver_index(), |_| ())
                .is_ok()
        );

        Ok(())
    }

    #[tokio::test]
    async fn a_one_way_opener_from_a_known_node_is_refused_without_gateway_state()
    -> anyhow::Result<()> {
        let (client_peer, exit) = mock_peers();
        let state = shared_node_state(exit);
        let known_node = Some(lp_node_details(client_peer.as_remote()));

        let mut client = serve_one_way_opener(state.clone(), None, client_peer, known_node).await?;

        // the handshake itself completes, then the handler returns and closes the connection
        client.handler.await???;
        let mut buf = [0u8; 1];
        assert_eq!(client.stream.read(&mut buf).await?, 0);

        assert_eq!(
            state
                .sessions
                .sending_index_for(LpPeer::node(client.remote_addr.ip())),
            None
        );
        assert!(
            state
                .sessions
                .with_session_mut(client.session.receiver_index(), |_| ())
                .is_err()
        );

        Ok(())
    }

    #[tokio::test]
    async fn the_client_connection_limit_applies_to_forwarded_clients() -> anyhow::Result<()> {
        let (client_peer, exit) = mock_peers();
        let mut state = shared_node_state(exit);
        state.lp_config.debug.max_connections = 0;
        let gateway = gateway_state(&state);
        let known_node = Some(lp_node_details(client_peer.as_remote()));

        let mut client =
            serve_one_way_opener(state.clone(), Some(gateway), client_peer, known_node).await?;

        client.handler.await???;
        let mut buf = [0u8; 1];
        assert_eq!(client.stream.read(&mut buf).await?, 0);

        assert_eq!(
            state
                .sessions
                .sending_index_for(LpPeer::node(client.remote_addr.ip())),
            None
        );
        assert!(
            state
                .sessions
                .with_session_mut(client.session.receiver_index(), |_| ())
                .is_err()
        );

        Ok(())
    }

    /// The limit reads the active-client gauge, so a plain client must count from its handshake on,
    /// and must stop counting however its handler ends.
    #[tokio::test]
    async fn a_plain_client_is_counted_from_its_handshake_on() -> anyhow::Result<()> {
        let (_, exit) = mock_peers();
        let state = shared_node_state(exit);
        let gateway = gateway_state(&state);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let stream = TcpStream::connect(listener.local_addr()?).await?;
        let (accepted, remote_addr) = listener.accept().await?;

        let handler = LpIngressConnectionHandler::new(
            accepted,
            remote_addr,
            None,
            state.clone(),
            Some(gateway),
        )
        .handle()
        .spawn_timeboxed();

        // let the handler run up to its first read: the client has sent nothing yet
        tokio::task::yield_now().await;
        assert_eq!(
            state.metrics.network.active_lp_client_connections_count(),
            1
        );

        // the client goes away without ever handshaking
        drop(stream);
        handler.await???;
        assert_eq!(
            state.metrics.network.active_lp_client_connections_count(),
            0
        );

        Ok(())
    }

    /// A plain client: the connecting address is nobody the directory knows.
    #[tokio::test]
    async fn a_one_way_opener_from_an_unknown_address_is_served_as_a_client() -> anyhow::Result<()>
    {
        let (client_peer, exit) = mock_peers();
        let state = shared_node_state(exit);
        let gateway = gateway_state(&state);

        let client = serve_one_way_opener(state.clone(), Some(gateway), client_peer, None).await?;

        drop(client.stream);
        client.handler.await???;

        assert!(
            state
                .sessions
                .with_session_mut(client.session.receiver_index(), |_| ())
                .is_ok()
        );

        Ok(())
    }

    /// A client cannot pass as a node: without the directory's digests a mutual opener fails.
    #[tokio::test]
    async fn a_mutual_opener_from_an_unknown_address_is_not_a_node_session() -> anyhow::Result<()> {
        let (client_peer, exit) = mock_peers();
        let state = shared_node_state(exit);
        let gateway = gateway_state(&state);
        let exit_remote = state.local_lp_peer.as_remote();

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let mut stream = TcpStream::connect(listener.local_addr()?).await?;
        let (accepted, remote_addr) = listener.accept().await?;

        let handler = LpIngressConnectionHandler::new(
            accepted,
            remote_addr,
            None,
            state.clone(),
            Some(gateway),
        )
        .handle()
        .spawn_timeboxed();

        let client_handshake = LpTransportSession::psq_handshake_initiator_mutual_internode(
            &mut stream,
            client_peer,
            exit_remote,
            version::CURRENT,
        )?
        .complete_handshake()
        .timeboxed()
        .await?;
        assert!(client_handshake.is_err());

        // a client's failed handshake is routine for the node, not an error
        handler.await???;
        assert_eq!(
            state
                .sessions
                .sending_index_for(LpPeer::node(remote_addr.ip())),
            None
        );

        Ok(())
    }

    /// A mutual opener proves nothing about the initiator's KEM key beyond knowing its public half.
    ///
    /// The KKT step only checks that the encapsulation key the opener carries hashes to the digest
    /// the directory lists for the node at the connecting address; the initiator's KEM private key
    /// is never used by the handshake. Every client learns an entry's public KEM key from its own
    /// KKT response with that entry, and the entry forwards whatever a client asks it to, so a
    /// client can send any exit a mutual opener carrying the entry's KEM key and its own x25519 key,
    /// arriving from the entry's address. The handshake completes. Without a further check the
    /// exit would store that session as the entry's node session, demoting the real one, and
    /// everything it then sent to the entry would go to the client instead.
    ///
    /// What does authenticate the initiator is the long-term x25519 key the PSQ handshake proves
    /// possession of, which the directory lists for the node as well. The exit must compare the
    /// two and refuse the opener on a mismatch, rather than storing it or serving it as a client.
    #[tokio::test]
    async fn a_mutual_opener_with_the_node_kem_key_but_another_x25519_key_is_refused()
    -> anyhow::Result<()> {
        let mut rng = deterministic_rng();
        let kem_keys = KEMKeys::new(
            generate_keypair_mceliece(&mut rng),
            generate_keypair_mlkem(&mut rng),
        );
        let entry = LpLocalPeer::new(
            Ciphersuite::default(),
            Arc::new(generate_lp_keypair_x25519(&mut rng)),
        )
        .with_kem_keys(kem_keys.clone());
        let impostor = LpLocalPeer::new(
            Ciphersuite::default(),
            Arc::new(generate_lp_keypair_x25519(&mut rng)),
        )
        .with_kem_keys(kem_keys);

        let (_, exit) = mock_peers();
        let state = shared_node_state(exit);
        let exit_remote = state.local_lp_peer.as_remote();

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let mut stream = TcpStream::connect(listener.local_addr()?).await?;
        let (accepted, remote_addr) = listener.accept().await?;

        let handler = LpIngressConnectionHandler::new(
            accepted,
            remote_addr,
            Some(lp_node_details(entry.as_remote())),
            state.clone(),
            Some(gateway_state(&state)),
        )
        .handle()
        .spawn_timeboxed();

        // the handshake itself completes: the KEM key matches the entry's digests
        let session = LpTransportSession::psq_handshake_initiator_mutual_internode(
            &mut stream,
            impostor,
            exit_remote,
            version::CURRENT,
        )?
        .complete_handshake()
        .timeboxed()
        .await??;

        // but it is nobody's node session, and the connection is refused rather than served
        assert!(handler.await??.is_err());
        assert_eq!(
            state
                .sessions
                .sending_index_for(LpPeer::node(remote_addr.ip())),
            None
        );
        assert!(
            state
                .sessions
                .with_session_mut(session.receiver_index(), |_| ())
                .is_err()
        );

        Ok(())
    }
}
