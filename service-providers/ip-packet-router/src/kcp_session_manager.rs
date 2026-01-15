// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![allow(unused)]

//! KCP Session Manager for LP clients at the exit gateway.
//!
//! This module sits between Sphinx unwrapping and IPR message processing.
//! It maintains per-client KCP state (keyed by conv_id from KCP packets),
//! reassembles KCP fragments into complete messages, and wraps responses
//! in KCP for SURB replies.
//!
//! # Architecture
//!
//! ```text
//! Mixnet → [Sphinx unwrap] → [KCP Session Manager] → [IPR message handling]
//!                                    ↓
//!                           KCP sessions per conv_id
//!                                    ↓
//!                           Reassemble fragments → DataRequest
//! ```
//!
//! # Design Notes
//!
//! - Conv ID is extracted from the first 4 bytes of KCP packet data
//! - SURBs are stored per conv_id for sending replies
//! - Pattern follows `nym-lp-node::Node` from lewes-protocol

use bytes::BytesMut;
use nym_kcp::driver::KcpDriver;
use nym_kcp::packet::KcpPacket;
use nym_kcp::session::KcpSession;
use nym_sphinx::anonymous_replies::ReplySurb;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::error::IpPacketRouterError;

/// Default session timeout (5 minutes, matching IPR client timeout)
const SESSION_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum concurrent KCP sessions per exit gateway
const MAX_SESSIONS: usize = 10000;

/// State for a single KCP session
struct KcpSessionState {
    driver: KcpDriver,
    /// SURBs for sending replies back to this client
    surbs: VecDeque<ReplySurb>,
    /// Last activity timestamp
    last_activity: Instant,
    /// The sender tag associated with this session (for logging/debugging)
    sender_tag: Option<AnonymousSenderTag>,
}

impl KcpSessionState {
    fn new(conv_id: u32) -> Self {
        let session = KcpSession::new(conv_id);
        Self {
            driver: KcpDriver::new(session),
            surbs: VecDeque::new(),
            last_activity: Instant::now(),
            sender_tag: None,
        }
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    fn is_expired(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    fn add_surbs(&mut self, surbs: Vec<ReplySurb>) {
        self.surbs.extend(surbs);
    }

    fn take_surb(&mut self) -> Option<ReplySurb> {
        self.surbs.pop_front()
    }

    fn surb_count(&self) -> usize {
        self.surbs.len()
    }
}

/// KCP Session Manager maintains per-client KCP state for LP clients.
///
/// It intercepts incoming Sphinx payloads containing KCP data, extracts KCP frames,
/// reassembles them into complete messages, and returns the assembled data for IPR processing.
///
/// Sessions are keyed by `conv_id` (first 4 bytes of KCP packet), which is derived
/// by clients from their local and remote addresses.
pub struct KcpSessionManager {
    /// KCP sessions keyed by conv_id (from KCP packet header)
    sessions: HashMap<u32, KcpSessionState>,
    /// Session timeout duration
    timeout: Duration,
    /// Maximum number of sessions
    max_sessions: usize,
}

/// Result of processing incoming KCP data from a client
pub(crate) struct KcpProcessingResult {
    /// The conv_id extracted from the KCP packet
    pub(crate) conversation_id: u32,

    /// Vector of decoded KCP packets (for inspection/logging)
    pub(crate) decoded_packets: Vec<KcpPacket>,

    /// Vector of complete reassembled messages ready for IPR processing
    pub(crate) reassembled_messages: Vec<Vec<u8>>,
}

impl Default for KcpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KcpSessionManager {
    /// Create a new KCP Session Manager with default settings
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            timeout: SESSION_TIMEOUT,
            max_sessions: MAX_SESSIONS,
        }
    }

    /// Create a new KCP Session Manager with custom settings
    pub fn with_config(timeout: Duration, max_sessions: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            timeout,
            max_sessions,
        }
    }

    /// Process incoming KCP data from a client.
    ///
    /// Takes raw KCP-encoded data (from a RepliableMessage). The conv_id is extracted
    /// from the first 4 bytes of the KCP data.
    ///
    /// # Arguments
    /// * `kcp_data` - Raw KCP packet data (conv_id is in first 4 bytes)
    /// * `reply_surbs` - SURBs attached to the message for sending replies
    /// * `sender_tag` - The anonymous sender tag (for logging/association)
    /// * `current_time_ms` - Current time in milliseconds for KCP timing
    ///
    /// # Returns
    /// A tuple containing:
    /// - The conv_id extracted from the KCP packet
    /// - A vector of decoded KCP packets (for inspection/logging)
    /// - A vector of complete reassembled messages ready for IPR processing
    pub fn process_incoming(
        &mut self,
        kcp_data: &[u8],
        reply_surbs: Vec<ReplySurb>,
        sender_tag: Option<AnonymousSenderTag>,
        current_time_ms: u64,
    ) -> Result<KcpProcessingResult, IpPacketRouterError> {
        if kcp_data.len() < 4 {
            return Err(IpPacketRouterError::KcpError(
                "KCP data too short to contain conv_id".to_string(),
            ));
        }

        // Extract conv_id from first 4 bytes of KCP packet
        let conv_id = u32::from_le_bytes([kcp_data[0], kcp_data[1], kcp_data[2], kcp_data[3]]);

        // Get or create session
        self.ensure_session(conv_id, sender_tag)?;

        let session = self
            .sessions
            .get_mut(&conv_id)
            .ok_or_else(|| IpPacketRouterError::KcpError("Session not found".to_string()))?;

        session.touch();

        // Store SURBs for later replies
        session.add_surbs(reply_surbs);

        // Input the KCP data and get decoded packets
        let decoded_packets = match session.driver.input(kcp_data) {
            Ok(pkts) => pkts,
            Err(e) => {
                log::warn!("KCP input error for conv_id {}: {}", conv_id, e);
                return Err(IpPacketRouterError::KcpError(e.to_string()));
            }
        };

        // Update KCP state machine
        session.driver.update(current_time_ms);

        // Collect any complete messages
        let incoming_messages: Vec<Vec<u8>> = session
            .driver
            .fetch_incoming()
            .into_iter()
            .map(|buf| buf.to_vec())
            .collect();

        Ok(KcpProcessingResult {
            conversation_id: conv_id,
            decoded_packets,
            reassembled_messages: incoming_messages,
        })
    }

    /// Wrap outgoing data in KCP for sending via SURB.
    ///
    /// # Arguments
    /// * `conv_id` - The conversation ID
    /// * `data` - The data to wrap in KCP
    /// * `current_time_ms` - Current time in milliseconds for KCP timing
    ///
    /// # Returns
    /// KCP-encoded packets ready to send
    pub fn wrap_response(
        &mut self,
        conv_id: u32,
        data: &[u8],
        current_time_ms: u64,
    ) -> Result<Vec<u8>, IpPacketRouterError> {
        let session = self
            .sessions
            .get_mut(&conv_id)
            .ok_or_else(|| IpPacketRouterError::KcpError("No session for conv_id".to_string()))?;

        session.touch();

        // Queue the data for sending
        session.driver.send(data);

        // Update to generate outgoing packets
        session.driver.update(current_time_ms);

        // Fetch outgoing KCP packets and encode
        let packets = session.driver.fetch_outgoing();
        let mut buf = BytesMut::new();
        for pkt in packets {
            pkt.encode(&mut buf);
        }

        Ok(buf.to_vec())
    }

    /// Take a SURB for sending a reply to a client.
    ///
    /// # Arguments
    /// * `conv_id` - The conversation ID
    ///
    /// # Returns
    /// A SURB if available, None otherwise
    pub fn take_surb(&mut self, conv_id: u32) -> Option<ReplySurb> {
        self.sessions.get_mut(&conv_id)?.take_surb()
    }

    /// Get the number of available SURBs for a session
    pub fn surb_count(&self, conv_id: u32) -> usize {
        self.sessions
            .get(&conv_id)
            .map(|s| s.surb_count())
            .unwrap_or(0)
    }

    /// Get the sender_tag associated with a session.
    ///
    /// Returns None if the session doesn't exist or has no sender_tag.
    pub fn get_sender_tag(&self, conv_id: u32) -> Option<AnonymousSenderTag> {
        self.sessions.get(&conv_id)?.sender_tag
    }

    /// Fetch any pending outgoing KCP packets for a specific session.
    ///
    /// This is used to send immediate ACKs after receiving packets,
    /// rather than waiting for the periodic tick.
    pub fn fetch_outgoing_for_conv(
        &mut self,
        conv_id: u32,
        current_time_ms: u64,
    ) -> Option<Vec<u8>> {
        let session = self.sessions.get_mut(&conv_id)?;
        session.driver.update(current_time_ms);
        let packets = session.driver.fetch_outgoing();

        if packets.is_empty() {
            return None;
        }

        let mut buf = BytesMut::new();
        for pkt in packets {
            pkt.encode(&mut buf);
        }
        Some(buf.to_vec())
    }

    /// Periodic update for all sessions.
    ///
    /// This should be called periodically (e.g., every 10-100ms) to:
    /// - Drive KCP state machines (retransmissions, etc.)
    /// - Clean up expired sessions
    ///
    /// Returns a list of (conv_id, outgoing_data) pairs for any sessions
    /// that have pending outgoing packets.
    pub fn tick(&mut self, current_time_ms: u64) -> Vec<(u32, Vec<u8>)> {
        let mut outgoing = Vec::new();

        for (&conv_id, session) in self.sessions.iter_mut() {
            session.driver.update(current_time_ms);
            let packets = session.driver.fetch_outgoing();

            if !packets.is_empty() {
                let mut buf = BytesMut::new();
                for pkt in packets {
                    pkt.encode(&mut buf);
                }
                outgoing.push((conv_id, buf.to_vec()));
            }
        }

        // Clean up expired sessions
        self.cleanup_expired();

        outgoing
    }

    /// Remove expired sessions.
    pub fn cleanup_expired(&mut self) {
        let timeout = self.timeout;
        self.sessions.retain(|conv_id, state| {
            let expired = state.is_expired(timeout);
            if expired {
                log::debug!("Removing expired KCP session for conv_id {}", conv_id);
            }
            !expired
        });
    }

    /// Get the number of active sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Check if a session exists for the given conv_id
    pub fn has_session(&self, conv_id: u32) -> bool {
        self.sessions.contains_key(&conv_id)
    }

    /// Ensure a session exists for the given conv_id, creating one if needed
    fn ensure_session(
        &mut self,
        conv_id: u32,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Result<(), IpPacketRouterError> {
        if self.sessions.contains_key(&conv_id) {
            // Update sender_tag if provided
            if let Some(tag) = sender_tag
                && let Some(session) = self.sessions.get_mut(&conv_id)
            {
                session.sender_tag = Some(tag);
            }

            return Ok(());
        }

        // Check session limit
        if self.sessions.len() >= self.max_sessions {
            // Try to clean up expired sessions first
            self.cleanup_expired();

            // Still at limit?
            if self.sessions.len() >= self.max_sessions {
                return Err(IpPacketRouterError::KcpError(
                    "Maximum KCP sessions reached".to_string(),
                ));
            }
        }

        log::debug!("Creating new KCP session for conv_id {}", conv_id);
        let mut state = KcpSessionState::new(conv_id);
        state.sender_tag = sender_tag;
        self.sessions.insert(conv_id, state);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let mut manager = KcpSessionManager::new();

        assert!(!manager.has_session(12345));
        assert_eq!(manager.session_count(), 0);

        // Create a minimal KCP packet (just conv_id)
        let conv_id: u32 = 12345;
        let mut kcp_data = conv_id.to_le_bytes().to_vec();
        // Add minimal header padding to make it look like a packet
        kcp_data.extend_from_slice(&[0u8; 21]); // KCP header is 25 bytes total

        // Processing data should create a session
        let result = manager.process_incoming(&kcp_data, vec![], None, 0);
        // May error due to invalid KCP packet, but session should be created
        let _ = result;

        assert!(manager.has_session(conv_id));
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_session_expiry() {
        let mut manager = KcpSessionManager::with_config(Duration::from_millis(10), 100);
        let conv_id: u32 = 99999;

        // Create session directly
        manager.ensure_session(conv_id, None).unwrap();
        assert!(manager.has_session(conv_id));

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(20));

        manager.cleanup_expired();
        assert!(!manager.has_session(conv_id));
    }

    #[test]
    fn test_max_sessions_limit() {
        let mut manager = KcpSessionManager::with_config(Duration::from_secs(300), 2);

        manager.ensure_session(1, None).unwrap();
        manager.ensure_session(2, None).unwrap();

        assert_eq!(manager.session_count(), 2);

        // Third session should fail
        let result = manager.ensure_session(3, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_kcp_roundtrip() {
        use nym_kcp::driver::KcpDriver;
        use nym_kcp::session::KcpSession;

        let mut manager = KcpSessionManager::new();
        let conv_id: u32 = 42424242;

        // Create a "client" KCP session to send data
        let client_session = KcpSession::new(conv_id);
        let mut client_driver = KcpDriver::new(client_session);

        // Client sends a message
        let message = b"Hello, IPR via KCP!";
        client_driver.send(message);
        client_driver.update(100);

        // Get the KCP packets from the client
        let outgoing = client_driver.fetch_outgoing();
        assert!(!outgoing.is_empty(), "Client should produce KCP packets");

        // Encode packets
        let mut kcp_data = BytesMut::new();
        for pkt in outgoing {
            pkt.encode(&mut kcp_data);
        }

        // Feed to the session manager
        let res = manager
            .process_incoming(&kcp_data, vec![], None, 100)
            .expect("process_incoming should succeed");

        // Verify conv_id was extracted correctly
        assert_eq!(res.conversation_id, conv_id);

        // Should have received the complete message
        assert_eq!(res.reassembled_messages.len(), 1);
        assert_eq!(res.reassembled_messages[0], message);
    }

    #[test]
    fn test_surb_storage() {
        let mut manager = KcpSessionManager::new();
        let conv_id: u32 = 11111;

        // Create session
        manager.ensure_session(conv_id, None).unwrap();

        // Initially no SURBs
        assert_eq!(manager.surb_count(conv_id), 0);
        assert!(manager.take_surb(conv_id).is_none());

        // Note: We can't easily create ReplySurbs in tests without complex setup,
        // but the storage mechanism is tested via the session state
    }
}
