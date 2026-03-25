// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session lifecycle management functionality, handling
//! creation, retrieval, and storage of sessions.

use crate::packet::{EncryptedLpPacket, LpFrame};
use crate::peer_config::LpReceiverIndex;
use crate::{LpError, LpTransportSession};
use std::collections::HashMap;

pub use crate::replay::validator::PacketCount;
use crate::session::{LpAction, LpInput};

/// Manages the lifecycle of Lewes Protocol sessions.
///
/// The SessionManager is responsible for creating, storing, and retrieving sessions
#[derive(Default)]
pub struct SessionManager {
    /// Manages state machines directly, keyed by lp_id
    sessions: HashMap<LpReceiverIndex, LpTransportSession>,
}

impl SessionManager {
    /// Creates a new session manager with empty session storage.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn process_input(
        &mut self,
        lp_id: LpReceiverIndex,
        input: LpInput,
    ) -> Result<LpAction, LpError> {
        self.with_session_mut(lp_id, |sm| sm.process_input(input))?
    }

    pub fn send_frame(
        &mut self,
        lp_id: LpReceiverIndex,
        frame: LpFrame,
    ) -> Result<LpAction, LpError> {
        self.process_input(lp_id, LpInput::SendFrame(frame))
    }

    pub fn receive_packet(
        &mut self,
        lp_id: LpReceiverIndex,
        packet: EncryptedLpPacket,
    ) -> Result<LpAction, LpError> {
        self.process_input(lp_id, LpInput::ReceivePacket(packet))
    }

    #[cfg(test)]
    fn get_session_id(&self, lp_id: LpReceiverIndex) -> Result<LpReceiverIndex, LpError> {
        self.with_session(lp_id, |sm| sm.receiver_index())
    }

    pub fn current_packet_cnt(&self, lp_id: LpReceiverIndex) -> Result<PacketCount, LpError> {
        self.with_session(lp_id, |sm| Ok(sm.current_packet_cnt()))?
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn session_exists(&self, lp_id: LpReceiverIndex) -> bool {
        self.sessions.contains_key(&lp_id)
    }

    pub fn with_session<F, R>(&self, lp_id: LpReceiverIndex, f: F) -> Result<R, LpError>
    where
        F: FnOnce(&LpTransportSession) -> R,
    {
        if let Some(sm) = self.sessions.get(&lp_id) {
            Ok(f(sm))
        } else {
            Err(LpError::StateMachineNotFound(lp_id))
        }
    }

    // For mutable access (like running process_input)
    pub fn with_session_mut<F, R>(&mut self, lp_id: LpReceiverIndex, f: F) -> Result<R, LpError>
    where
        F: FnOnce(&mut LpTransportSession) -> R, // Closure takes mutable ref
    {
        if let Some(sm) = self.sessions.get_mut(&lp_id) {
            Ok(f(sm))
        } else {
            Err(LpError::StateMachineNotFound(lp_id))
        }
    }

    pub fn insert_session(
        &mut self,
        lp_session: LpTransportSession,
    ) -> Result<LpReceiverIndex, LpError> {
        let session_id = lp_session.receiver_index();

        if self.sessions.contains_key(&session_id) {
            return Err(LpError::DuplicateSessionId(session_id));
        }

        self.sessions.insert(session_id, lp_session);
        Ok(session_id)
    }

    /// Method to remove a state machine
    pub fn remove_session(&mut self, lp_id: LpReceiverIndex) -> bool {
        let removed = self.sessions.remove(&lp_id);

        removed.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SessionsMock, mock_session_for_test};
    use nym_kkt_ciphersuite::{IntoEnumIterator, KEM};

    #[test]
    fn test_session_manager_get() {
        let mut manager = SessionManager::new();
        let local_session = mock_session_for_test();
        let id = local_session.receiver_index();

        let sm_1_id = manager.insert_session(local_session).unwrap();
        assert_eq!(sm_1_id, id);

        let retrieved = manager.session_exists(id);
        assert!(retrieved);

        let not_found = manager.session_exists(123);
        assert!(!not_found);
    }

    #[test]
    fn test_session_manager_remove() {
        let mut manager = SessionManager::new();
        let local_session = mock_session_for_test();
        let sm_1_id = manager.insert_session(local_session).unwrap();

        let removed = manager.remove_session(sm_1_id);
        assert!(removed);
        assert_eq!(manager.session_count(), 0);

        let removed_again = manager.remove_session(sm_1_id);
        assert!(!removed_again);
    }

    #[test]
    fn test_multiple_sessions() {
        for kem in KEM::iter() {
            let mut manager = SessionManager::new();
            let session1 = SessionsMock::mock_seeded_post_handshake(123, kem).initiator;
            let session2 = SessionsMock::mock_seeded_post_handshake(124, kem).initiator;
            let session3 = SessionsMock::mock_seeded_post_handshake(125, kem).initiator;

            let sm_1 = manager.insert_session(session1).unwrap();
            let sm_2 = manager.insert_session(session2).unwrap();
            let sm_3 = manager.insert_session(session3).unwrap();

            assert_eq!(manager.session_count(), 3);

            let retrieved1 = manager.get_session_id(sm_1).unwrap();
            let retrieved2 = manager.get_session_id(sm_2).unwrap();
            let retrieved3 = manager.get_session_id(sm_3).unwrap();

            assert_eq!(retrieved1, sm_1);
            assert_eq!(retrieved2, sm_2);
            assert_eq!(retrieved3, sm_3);
        }
    }

    #[test]
    fn test_session_manager_create_session() {
        let mut manager = SessionManager::new();

        let sesion = mock_session_for_test();

        let sm = manager.insert_session(sesion).unwrap();
        assert_eq!(manager.session_count(), 1);

        let retrieved = manager.get_session_id(sm);
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), sm);
    }
}
