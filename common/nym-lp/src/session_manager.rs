// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session lifecycle management functionality, handling
//! creation, retrieval, and storage of sessions.

use crate::noise_protocol::ReadResult;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::state_machine::{LpAction, LpInput, LpState, LpStateBare};
use crate::{LpError, LpMessage, LpSession, LpStateMachine};
use dashmap::DashMap;

/// Manages the lifecycle of Lewes Protocol sessions.
///
/// The SessionManager is responsible for creating, storing, and retrieving sessions,
/// ensuring proper thread-safety for concurrent access.
pub struct SessionManager {
    /// Manages state machines directly, keyed by lp_id
    state_machines: DashMap<u32, LpStateMachine>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Creates a new session manager with empty session storage.
    pub fn new() -> Self {
        Self {
            state_machines: DashMap::new(),
        }
    }

    pub fn process_input(&self, lp_id: u32, input: LpInput) -> Result<Option<LpAction>, LpError> {
        self.with_state_machine_mut(lp_id, |sm| sm.process_input(input).transpose())?
    }

    pub fn add(&self, session: LpSession) -> Result<(), LpError> {
        let sm = LpStateMachine {
            state: LpState::ReadyToHandshake {
                session: Box::new(session),
            },
        };
        self.state_machines.insert(sm.id()?, sm);
        Ok(())
    }

    pub fn handshaking(&self, lp_id: u32) -> Result<bool, LpError> {
        Ok(self.get_state(lp_id)? == LpStateBare::Handshaking)
    }

    pub fn should_initiate_handshake(&self, lp_id: u32) -> Result<bool, LpError> {
        Ok(self.ready_to_handshake(lp_id)? || self.closed(lp_id)?)
    }

    pub fn ready_to_handshake(&self, lp_id: u32) -> Result<bool, LpError> {
        Ok(self.get_state(lp_id)? == LpStateBare::ReadyToHandshake)
    }

    pub fn closed(&self, lp_id: u32) -> Result<bool, LpError> {
        Ok(self.get_state(lp_id)? == LpStateBare::Closed)
    }

    pub fn transport(&self, lp_id: u32) -> Result<bool, LpError> {
        Ok(self.get_state(lp_id)? == LpStateBare::Transport)
    }

    #[cfg(test)]
    fn get_state_machine_id(&self, lp_id: u32) -> Result<u32, LpError> {
        self.with_state_machine(lp_id, |sm| sm.id())?
    }

    pub fn get_state(&self, lp_id: u32) -> Result<LpStateBare, LpError> {
        self.with_state_machine(lp_id, |sm| Ok(sm.bare_state()))?
    }

    pub fn receiving_counter_quick_check(&self, lp_id: u32, counter: u64) -> Result<(), LpError> {
        self.with_state_machine(lp_id, |sm| {
            sm.session()?.receiving_counter_quick_check(counter)
        })?
    }

    pub fn receiving_counter_mark(&self, lp_id: u32, counter: u64) -> Result<(), LpError> {
        self.with_state_machine(lp_id, |sm| sm.session()?.receiving_counter_mark(counter))?
    }

    pub fn start_handshake(&self, lp_id: u32) -> Option<Result<LpMessage, LpError>> {
        self.prepare_handshake_message(lp_id)
    }

    pub fn prepare_handshake_message(&self, lp_id: u32) -> Option<Result<LpMessage, LpError>> {
        self.with_state_machine(lp_id, |sm| sm.session().ok()?.prepare_handshake_message())
            .ok()?
    }

    pub fn is_handshake_complete(&self, lp_id: u32) -> Result<bool, LpError> {
        self.with_state_machine(lp_id, |sm| Ok(sm.session()?.is_handshake_complete()))?
    }

    pub fn next_counter(&self, lp_id: u32) -> Result<u64, LpError> {
        self.with_state_machine(lp_id, |sm| Ok(sm.session()?.next_counter()))?
    }

    pub fn decrypt_data(&self, lp_id: u32, message: &LpMessage) -> Result<Vec<u8>, LpError> {
        self.with_state_machine(lp_id, |sm| {
            sm.session()?
                .decrypt_data(message)
                .map_err(LpError::NoiseError)
        })?
    }

    pub fn encrypt_data(&self, lp_id: u32, message: &[u8]) -> Result<LpMessage, LpError> {
        self.with_state_machine(lp_id, |sm| {
            sm.session()?
                .encrypt_data(message)
                .map_err(LpError::NoiseError)
        })?
    }

    pub fn current_packet_cnt(&self, lp_id: u32) -> Result<(u64, u64), LpError> {
        self.with_state_machine(lp_id, |sm| Ok(sm.session()?.current_packet_cnt()))?
    }

    pub fn process_handshake_message(
        &self,
        lp_id: u32,
        message: &LpMessage,
    ) -> Result<ReadResult, LpError> {
        self.with_state_machine(lp_id, |sm| sm.session()?.process_handshake_message(message))?
    }

    pub fn session_count(&self) -> usize {
        self.state_machines.len()
    }

    pub fn state_machine_exists(&self, lp_id: u32) -> bool {
        self.state_machines.contains_key(&lp_id)
    }

    pub fn with_state_machine<F, R>(&self, lp_id: u32, f: F) -> Result<R, LpError>
    where
        F: FnOnce(&LpStateMachine) -> R,
    {
        if let Some(sm) = self.state_machines.get(&lp_id) {
            Ok(f(&sm))
        } else {
            Err(LpError::StateMachineNotFound { lp_id })
        }
        // self.state_machines.get(&lp_id).map(|sm_ref| f(&*sm_ref)) // Lock held only during closure execution
    }

    // For mutable access (like running process_input)
    pub fn with_state_machine_mut<F, R>(&self, lp_id: u32, f: F) -> Result<R, LpError>
    where
        F: FnOnce(&mut LpStateMachine) -> R, // Closure takes mutable ref
    {
        if let Some(mut sm) = self.state_machines.get_mut(&lp_id) {
            Ok(f(&mut sm))
        } else {
            Err(LpError::StateMachineNotFound { lp_id })
        }
    }

    pub fn create_session_state_machine(
        &self,
        receiver_index: u32,
        is_initiator: bool,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
        salt: &[u8; 32],
    ) -> Result<u32, LpError> {
        let sm = LpStateMachine::new(receiver_index, is_initiator, local_peer, remote_peer, salt)?;

        self.state_machines.insert(receiver_index, sm);
        Ok(receiver_index)
    }

    /// Method to remove a state machine
    pub fn remove_state_machine(&self, lp_id: u32) -> bool {
        let removed = self.state_machines.remove(&lp_id);

        removed.is_some()
    }

    /// Test-only method to initialize KKT state to Completed for a session.
    /// This allows integration tests to bypass KKT exchange and directly test PSQ/handshake.
    #[cfg(test)]
    pub fn init_kkt_for_test(
        &self,
        lp_id: u32,
        remote_x25519_pub: &nym_crypto::asymmetric::x25519::PublicKey,
    ) -> Result<(), LpError> {
        self.with_state_machine(lp_id, |sm| {
            sm.session()?.set_kkt_completed_for_test(remote_x25519_pub);
            Ok(())
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{mock_peers, random_peer};
    use nym_test_utils::helpers::deterministic_rng;

    #[test]
    fn test_session_manager_get() {
        let manager = SessionManager::new();
        let mut rng = deterministic_rng();
        let local = random_peer(&mut rng);
        let peer1 = random_peer(&mut rng);

        let salt = [47u8; 32];
        let receiver_index: u32 = 1001;

        let sm_1_id = manager
            .create_session_state_machine(receiver_index, true, local, peer1.as_remote(), &salt)
            .unwrap();

        let retrieved = manager.state_machine_exists(sm_1_id);
        assert!(retrieved);

        let not_found = manager.state_machine_exists(99);
        assert!(!not_found);
    }

    #[test]
    fn test_session_manager_remove() {
        let manager = SessionManager::new();
        let mut rng = deterministic_rng();
        let local = random_peer(&mut rng);
        let peer1 = random_peer(&mut rng);

        let salt = [48u8; 32];
        let receiver_index: u32 = 2002;

        let sm_1_id = manager
            .create_session_state_machine(receiver_index, true, local, peer1.as_remote(), &salt)
            .unwrap();

        let removed = manager.remove_state_machine(sm_1_id);
        assert!(removed);
        assert_eq!(manager.session_count(), 0);

        let removed_again = manager.remove_state_machine(sm_1_id);
        assert!(!removed_again);
    }

    #[test]
    fn test_multiple_sessions() {
        let manager = SessionManager::new();
        let mut rng = deterministic_rng();
        let local = random_peer(&mut rng);
        let peer1 = random_peer(&mut rng);
        let peer2 = random_peer(&mut rng);
        let peer3 = random_peer(&mut rng);

        let salt = [49u8; 32];

        let sm_1 = manager
            .create_session_state_machine(3001, true, local.clone(), peer1.as_remote(), &salt)
            .unwrap();

        let sm_2 = manager
            .create_session_state_machine(3002, true, local.clone(), peer2.as_remote(), &salt)
            .unwrap();

        let sm_3 = manager
            .create_session_state_machine(3003, true, local.clone(), peer3.as_remote(), &salt)
            .unwrap();

        assert_eq!(manager.session_count(), 3);

        let retrieved1 = manager.get_state_machine_id(sm_1).unwrap();
        let retrieved2 = manager.get_state_machine_id(sm_2).unwrap();
        let retrieved3 = manager.get_state_machine_id(sm_3).unwrap();

        assert_eq!(retrieved1, sm_1);
        assert_eq!(retrieved2, sm_2);
        assert_eq!(retrieved3, sm_3);
    }

    #[test]
    fn test_session_manager_create_session() {
        let manager = SessionManager::new();
        let (init, resp) = mock_peers();

        let salt = [50u8; 32];
        let receiver_index: u32 = 4004;

        let sm = manager.create_session_state_machine(
            receiver_index,
            true,
            init,
            resp.as_remote(),
            &salt,
        );

        assert!(sm.is_ok());
        let sm = sm.unwrap();

        assert_eq!(manager.session_count(), 1);

        let retrieved = manager.get_state_machine_id(sm);
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), sm);
    }
}
