// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use crate::node::lp::SharedLpState;
    use crate::node::lp::control::egress::connection::InitialLpEgressNodeConnectionHandler;
    use crate::node::lp::control::ingress::node_handler::InitialLpIngressNodeConnectionHandler;
    use crate::node::lp::directory::LpNodeDetails;
    use crate::node::lp::state::SharedLpNodeControlState;
    use anyhow::Context;
    use nym_lp::packet::version;
    use nym_lp::peer::{LpLocalPeer, LpRemotePeer, mock_peers};
    use nym_test_utils::helpers::seeded_rng;
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_test_utils::traits::TimeboxedSpawnable;
    use rand::RngCore;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn shared_node_state(peer: LpLocalPeer) -> SharedLpNodeControlState {
        SharedLpNodeControlState {
            local_lp_peer: peer,
            nodes: Default::default(),
            shared: SharedLpState {
                metrics: Default::default(),
                lp_config: Default::default(),
                session_states: Default::default(),
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
        let init_result = init_result??.context("handshake failure")??;
        let resp_result = resp_result??.context("handshake failure")??;

        assert_eq!(
            init_result.receiver_index(),
            resp_result.transport_session().receiver_index()
        );

        Ok(())
    }
}
