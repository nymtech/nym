// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bincode::Options;
use serde::{Deserialize, Serialize};

use super::error::Error;
use crate::{
    make_bincode_serializer,
    models::{AvailableBandwidthResponse, TopUpRequest},
};

use nym_credentials_interface::CredentialSpendingData;

pub const VERSION: usize = 1;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InnerAvailableBandwidthResponse {
    pub(crate) value: i64,
}

impl InnerAvailableBandwidthResponse {
    pub(crate) fn new(value: i64) -> Self {
        Self { value }
    }
}

impl TryFrom<InnerAvailableBandwidthResponse> for AvailableBandwidthResponse {
    type Error = Error;

    fn try_from(value: InnerAvailableBandwidthResponse) -> Result<Self, Self::Error> {
        Ok(AvailableBandwidthResponse {
            version: VERSION,
            inner: make_bincode_serializer().serialize(&value)?,
        })
    }
}

impl TryFrom<AvailableBandwidthResponse> for InnerAvailableBandwidthResponse {
    type Error = Error;

    fn try_from(value: AvailableBandwidthResponse) -> Result<Self, Self::Error> {
        if value.version != VERSION {
            return Err(Error::InvalidVersion {
                source_version: value.version,
                target_version: VERSION,
            });
        }
        Ok(make_bincode_serializer().deserialize(&value.inner)?)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InnerTopUpRequest {
    /// Ecash credential
    pub credential: CredentialSpendingData,
}

impl TryFrom<InnerTopUpRequest> for TopUpRequest {
    type Error = Error;

    fn try_from(value: InnerTopUpRequest) -> Result<Self, Self::Error> {
        Ok(TopUpRequest {
            version: VERSION,
            inner: make_bincode_serializer().serialize(&value)?,
        })
    }
}

impl TryFrom<TopUpRequest> for InnerTopUpRequest {
    type Error = Error;

    fn try_from(value: TopUpRequest) -> Result<Self, Self::Error> {
        if value.version != VERSION {
            return Err(Error::InvalidVersion {
                source_version: value.version,
                target_version: VERSION,
            });
        }
        Ok(make_bincode_serializer().deserialize(&value.inner)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_available_bandwidth() {
        let bw = InnerAvailableBandwidthResponse::new(42);
        let ser = AvailableBandwidthResponse::try_from(bw).unwrap();
        assert_eq!(VERSION, ser.version);
        assert_eq!(ser.inner, vec![84]);
        let de = InnerAvailableBandwidthResponse::try_from(ser).unwrap();
        assert_eq!(bw, de);
    }

    #[test]
    fn mismatched_version_available_bandwidth() {
        let version = 4242;
        let future_bw = AvailableBandwidthResponse {
            version,
            inner: vec![],
        };
        if let Err(Error::InvalidVersion {
            source_version,
            target_version,
        }) = InnerAvailableBandwidthResponse::try_from(future_bw)
        {
            assert_eq!(source_version, version);
            assert_eq!(target_version, VERSION);
        } else {
            panic!("failed");
        };
    }

    #[test]
    fn invalid_content_available_bandwidth() {
        let future_bw = AvailableBandwidthResponse {
            version: VERSION,
            inner: vec![],
        };
        assert!(InnerAvailableBandwidthResponse::try_from(future_bw).is_err());
    }

    #[test]
    fn serde_topup() {
        let bw = InnerAvailableBandwidthResponse::new(42);
        let ser = AvailableBandwidthResponse::try_from(bw).unwrap();
        assert_eq!(VERSION, ser.version);
        assert_eq!(ser.inner, vec![84]);
        let de = InnerAvailableBandwidthResponse::try_from(ser).unwrap();
        assert_eq!(bw, de);
    }

    #[test]
    fn mismatched_version_topup() {
        let version = 4242;
        let future_bw = AvailableBandwidthResponse {
            version,
            inner: vec![],
        };
        if let Err(Error::InvalidVersion {
            source_version,
            target_version,
        }) = InnerAvailableBandwidthResponse::try_from(future_bw)
        {
            assert_eq!(source_version, version);
            assert_eq!(target_version, VERSION);
        } else {
            panic!("failed");
        };
    }

    #[test]
    fn invalid_content_topup() {
        let future_bw = AvailableBandwidthResponse {
            version: VERSION,
            inner: vec![],
        };
        assert!(InnerAvailableBandwidthResponse::try_from(future_bw).is_err());
    }
}
