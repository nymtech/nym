// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use crate::node::lp::active_sessions::{ActiveLpSessions, LpPeer};
    use crate::node::lp::control::egress::connection::InitialLpEgressNodeConnectionHandler;
    use crate::node::lp::control::ingress::node_handler::InitialLpIngressNodeConnectionHandler;
    use crate::node::lp::directory::LpNodeDetails;
    use crate::node::lp::state::SharedLpNodeControlState;
    use crate::node::lp::{SharedLpState, directory::LpNodes};
    use nym_lp::peer::{LpLocalPeer, LpRemotePeer, mock_peers};
    use nym_lp_data::packet::version;
    use nym_test_utils::helpers::seeded_rng;
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::TimeboxedSpawnable;
    use rand::RngCore;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn shared_node_state(peer: LpLocalPeer) -> SharedLpNodeControlState {
        SharedLpNodeControlState {
            local_lp_peer: peer,
            nodes: LpNodes::new_empty(),
            shared: SharedLpState {
                sessions: Default::default(),
                metrics: Default::default(),
                lp_config: Default::default(),
            },
        }
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

        let conn_init = MockIOStream::default();
        let conn_resp = conn_init.try_get_remote_handle();

        let init_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234);
        let init_details = lp_node_details(init_remote);
        let resp_details = lp_node_details(resp_remote);

        let init_state = shared_node_state(init);
        let resp_state = shared_node_state(resp);

        // keep handles so we can assert on what ended up stored
        let init_sessions = init_state.shared.sessions.clone();
        let resp_sessions = resp_state.shared.sessions.clone();

        let init_handler = InitialLpEgressNodeConnectionHandler::new(
            conn_init,
            init_addr,
            resp_details,
            init_state,
        );

        let resp_handler = InitialLpIngressNodeConnectionHandler::new(
            conn_resp,
            init_addr,
            init_details,
            resp_state,
        );

        let init_future = init_handler.complete_initial_handshake().spawn_timeboxed();
        let resp_future = resp_handler.complete_initial_handshake().spawn_timeboxed();

        let (init_result, resp_result) = tokio::join!(init_future, resp_future);
        let init_index = init_result???;
        let resp_index = resp_result???;

        // both sides derive the same receiver index for the session
        assert_eq!(init_index, resp_index);

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
}
