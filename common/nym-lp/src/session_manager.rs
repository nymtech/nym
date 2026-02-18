// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session lifecycle management functionality, handling
//! creation, retrieval, and storage of sessions.

use crate::session::SessionId;
use crate::state_machine::{LpAction, LpInput, LpStateBare};
use crate::{LpError, LpSession, LpStateMachine};
use std::collections::HashMap;

/// Manages the lifecycle of Lewes Protocol sessions.
///
/// The SessionManager is responsible for creating, storing, and retrieving sessions
pub struct SessionManager {
    /// Manages state machines directly, keyed by lp_id
    state_machines: HashMap<SessionId, LpStateMachine>,
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
            state_machines: HashMap::new(),
        }
    }

    pub fn process_input(
        &mut self,
        lp_id: SessionId,
        input: LpInput,
    ) -> Result<Option<LpAction>, LpError> {
        self.with_state_machine_mut(lp_id, |sm| sm.process_input(input).transpose())?
    }

    pub fn closed(&self, lp_id: SessionId) -> Result<bool, LpError> {
        Ok(self.get_state(lp_id)? == LpStateBare::Closed)
    }

    pub fn transport(&self, lp_id: SessionId) -> Result<bool, LpError> {
        Ok(self.get_state(lp_id)? == LpStateBare::Transport)
    }

    #[cfg(test)]
    fn get_state_machine_id(&self, lp_id: SessionId) -> Result<SessionId, LpError> {
        self.with_state_machine(lp_id, |sm| sm.id())?
    }

    pub fn get_state(&self, lp_id: SessionId) -> Result<LpStateBare, LpError> {
        self.with_state_machine(lp_id, |sm| Ok(sm.bare_state()))?
    }

    pub fn receiving_counter_quick_check(
        &self,
        lp_id: SessionId,
        counter: u64,
    ) -> Result<(), LpError> {
        self.with_state_machine(lp_id, |sm| {
            sm.session()?.receiving_counter_quick_check(counter)
        })?
    }

    pub fn receiving_counter_mark(
        &mut self,
        lp_id: SessionId,
        counter: u64,
    ) -> Result<(), LpError> {
        self.with_state_machine_mut(lp_id, |sm| {
            sm.session_mut()?.receiving_counter_mark(counter)
        })?
    }

    pub fn next_counter(&mut self, lp_id: SessionId) -> Result<u64, LpError> {
        self.with_state_machine_mut(lp_id, |sm| Ok(sm.session_mut()?.next_counter()))?
    }

    pub fn current_packet_cnt(&self, lp_id: SessionId) -> Result<(u64, u64), LpError> {
        self.with_state_machine(lp_id, |sm| Ok(sm.session()?.current_packet_cnt()))?
    }

    pub fn session_count(&self) -> usize {
        self.state_machines.len()
    }

    pub fn state_machine_exists(&self, lp_id: SessionId) -> bool {
        self.state_machines.contains_key(&lp_id)
    }

    pub fn with_state_machine<F, R>(&self, lp_id: SessionId, f: F) -> Result<R, LpError>
    where
        F: FnOnce(&LpStateMachine) -> R,
    {
        if let Some(sm) = self.state_machines.get(&lp_id) {
            Ok(f(sm))
        } else {
            Err(LpError::StateMachineNotFound { lp_id })
        }
    }

    // For mutable access (like running process_input)
    pub fn with_state_machine_mut<F, R>(&mut self, lp_id: SessionId, f: F) -> Result<R, LpError>
    where
        F: FnOnce(&mut LpStateMachine) -> R, // Closure takes mutable ref
    {
        if let Some(sm) = self.state_machines.get_mut(&lp_id) {
            Ok(f(sm))
        } else {
            Err(LpError::StateMachineNotFound { lp_id })
        }
    }

    pub fn create_session_state_machine(&mut self, lp_session: LpSession) -> SessionId {
        let session_id = *lp_session.session_identifier();
        let sm = LpStateMachine::new(lp_session);
        self.state_machines.insert(session_id, sm);
        session_id
    }

    /// Method to remove a state machine
    pub fn remove_state_machine(&mut self, lp_id: SessionId) -> bool {
        let removed = self.state_machines.remove(&lp_id);

        removed.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SessionsMock, mock_session_for_test};

    #[test]
    fn test_session_manager_get() {
        let mut manager = SessionManager::new();

        let TODO = "        for kem in kem_list() {";

        let local_session = mock_session_for_test();
        let id = local_session.id();

        let sm_1_id = manager.create_session_state_machine(local_session);
        assert_eq!(sm_1_id, id);

        let retrieved = manager.state_machine_exists(id);
        assert!(retrieved);

        let not_found = manager.state_machine_exists(99);
        assert!(!not_found);
    }

    #[test]
    fn test_session_manager_remove() {
        let mut manager = SessionManager::new();
        let local_session = mock_session_for_test();

        let TODO = "        for kem in kem_list() {";

        let sm_1_id = manager.create_session_state_machine(local_session);

        let removed = manager.remove_state_machine(sm_1_id);
        assert!(removed);
        assert_eq!(manager.session_count(), 0);

        let removed_again = manager.remove_state_machine(sm_1_id);
        assert!(!removed_again);
    }

    #[test]
    fn test_multiple_sessions() {
        let mut manager = SessionManager::new();

        let TODO = "        for kem in kem_list() {";

        let session1 = SessionsMock::mock_post_handshake(123).initiator;
        let session2 = SessionsMock::mock_post_handshake(124).initiator;
        let session3 = SessionsMock::mock_post_handshake(125).initiator;

        let sm_1 = manager.create_session_state_machine(session1);
        let sm_2 = manager.create_session_state_machine(session2);
        let sm_3 = manager.create_session_state_machine(session3);

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
        let mut manager = SessionManager::new();

        let TODO = "        for kem in kem_list() {";

        let sesion = mock_session_for_test();

        let sm = manager.create_session_state_machine(sesion);
        assert_eq!(manager.session_count(), 1);

        let retrieved = manager.get_state_machine_id(sm);
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), sm);
    }
}
