// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Error types for replay protection.

use thiserror::Error;

/// Errors that can occur during replay protection validation.
#[derive(Debug, Error)]
pub enum ReplayError {
    /// The counter value is invalid (e.g., too far in the future)
    #[error("Invalid counter value")]
    InvalidCounter,

    /// The packet has already been received (replay attack)
    #[error("Duplicate counter value")]
    DuplicateCounter,

    /// The packet is outside the replay window
    #[error("Packet outside replay window")]
    OutOfWindow,
}

/// Result type for replay protection operations
pub type ReplayResult<T> = Result<T, ReplayError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LpError;

    #[test]
    fn test_replay_error_variants() {
        let invalid = ReplayError::InvalidCounter;
        let duplicate = ReplayError::DuplicateCounter;
        let out_of_window = ReplayError::OutOfWindow;

        assert_eq!(invalid.to_string(), "Invalid counter value");
        assert_eq!(duplicate.to_string(), "Duplicate counter value");
        assert_eq!(out_of_window.to_string(), "Packet outside replay window");
    }

    #[test]
    fn test_replay_error_conversion() {
        let replay_error = ReplayError::InvalidCounter;
        let lp_error: LpError = replay_error.into();

        match lp_error {
            LpError::Replay(e) => {
                assert!(matches!(e, ReplayError::InvalidCounter));
            }
            _ => panic!("Expected Replay variant"),
        }
    }

    #[test]
    fn test_replay_result() {
        let ok_result: ReplayResult<()> = Ok(());
        let err_result: ReplayResult<()> = Err(ReplayError::InvalidCounter);

        assert!(ok_result.is_ok());
        assert!(err_result.is_err());
        assert!(matches!(
            err_result.unwrap_err(),
            ReplayError::InvalidCounter
        ));
    }
}
