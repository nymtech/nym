// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sdk::mixnet::ReconstructedMessage;

use crate::error::{Error, Result};

fn check_ipr_wire_reply_version(version: u8) -> Result<()> {
    if version == 8 || version == 9 {
        return Ok(());
    }
    if version < 8 {
        return Err(Error::ReceivedResponseWithOldVersion {
            expected: 8,
            received: version,
        });
    }
    Err(Error::ReceivedResponseWithNewVersion {
        expected: 9,
        received: version,
    })
}

/// IPR responses on the wire may be v8 or v9 (identical payload layout; version byte differs).
pub(crate) fn check_ipr_message_version(message: &ReconstructedMessage) -> Result<()> {
    let version = message
        .message
        .first()
        .copied()
        .ok_or(Error::NoVersionInMessage)?;
    check_ipr_wire_reply_version(version)
}

#[cfg(test)]
mod tests {
    use super::check_ipr_wire_reply_version;
    use crate::Error;

    #[test]
    fn wire_reply_accepts_v8_and_v9() {
        assert!(check_ipr_wire_reply_version(8).is_ok());
        assert!(check_ipr_wire_reply_version(9).is_ok());
    }

    #[test]
    fn wire_reply_rejects_older_than_v8() {
        let err = check_ipr_wire_reply_version(7).unwrap_err();
        match err {
            Error::ReceivedResponseWithOldVersion { expected, received } => {
                assert_eq!(expected, 8);
                assert_eq!(received, 7);
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn wire_reply_rejects_newer_than_v9() {
        let err = check_ipr_wire_reply_version(10).unwrap_err();
        match err {
            Error::ReceivedResponseWithNewVersion { expected, received } => {
                assert_eq!(expected, 9);
                assert_eq!(received, 10);
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    }
}
