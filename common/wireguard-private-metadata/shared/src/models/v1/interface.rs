// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::CredentialSpendingData;

#[cfg(feature = "testing")]
use super::super::v0 as previous;

use super::{
    available_bandwidth::{
        request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
    },
    topup_bandwidth::{request::InnerTopUpRequest, response::InnerTopUpResponse},
    QueryType, VersionedRequest, VersionedResponse, VERSION,
};
use crate::models::{error::Error, Construct, Extract, Version};

#[derive(Debug, Clone, PartialEq)]
pub enum RequestData {
    AvailableBandwidth(()),
    TopUpBandwidth(Box<CredentialSpendingData>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseData {
    AvailableBandwidth(i64),
    TopUpBandwidth(i64),
}

impl Construct<RequestData> for VersionedRequest {
    fn construct(info: RequestData, _version: Version) -> Result<Self, Error> {
        match info {
            RequestData::AvailableBandwidth(_) => Ok(InnerAvailableBandwidthRequest {}.try_into()?),
            RequestData::TopUpBandwidth(credential) => Ok(InnerTopUpRequest {
                credential: *credential,
            }
            .try_into()?),
        }
    }
}

impl Extract<RequestData> for VersionedRequest {
    fn extract(&self) -> Result<(RequestData, Version), Error> {
        match self.query_type {
            QueryType::AvailableBandwidth => {
                let _req = InnerAvailableBandwidthRequest::try_from(self.clone())?;
                Ok((RequestData::AvailableBandwidth(()), VERSION))
            }
            QueryType::TopupBandwidth => {
                let req = InnerTopUpRequest::try_from(self.clone())?;
                Ok((
                    RequestData::TopUpBandwidth(Box::new(req.credential)),
                    VERSION,
                ))
            }
        }
    }
}

impl Construct<ResponseData> for VersionedResponse {
    fn construct(info: ResponseData, _version: Version) -> Result<Self, Error> {
        match info {
            ResponseData::AvailableBandwidth(available_bandwidth) => {
                Ok(InnerAvailableBandwidthResponse {
                    available_bandwidth,
                }
                .try_into()?)
            }
            ResponseData::TopUpBandwidth(available_bandwidth) => Ok(InnerTopUpResponse {
                available_bandwidth,
            }
            .try_into()?),
        }
    }
}

impl Extract<ResponseData> for VersionedResponse {
    fn extract(&self) -> Result<(ResponseData, Version), Error> {
        match self.query_type {
            QueryType::AvailableBandwidth => {
                let resp = InnerAvailableBandwidthResponse::try_from(self.clone())?;
                Ok((
                    ResponseData::AvailableBandwidth(resp.available_bandwidth),
                    VERSION,
                ))
            }
            QueryType::TopupBandwidth => {
                let resp = InnerTopUpResponse::try_from(self.clone())?;
                Ok((
                    ResponseData::TopUpBandwidth(resp.available_bandwidth),
                    VERSION,
                ))
            }
        }
    }
}

// this should be with #[cfg(feature = "testing")] only coming from v0, don't copy this for future versions
#[cfg(feature = "testing")]
impl TryFrom<previous::interface::RequestData> for RequestData {
    type Error = super::Error;

    fn try_from(value: previous::interface::RequestData) -> Result<Self, Self::Error> {
        match value {
            previous::interface::RequestData::AvailableBandwidth(inner) => {
                Ok(Self::AvailableBandwidth(inner))
            }
            previous::interface::RequestData::TopUpBandwidth(_) => {
                Err(super::Error::UpdateNotPossible {
                    from: previous::VERSION,
                    to: VERSION,
                })
            }
        }
    }
}

// this should be with #[cfg(feature = "testing")] only coming from v0, don't copy this for future versions
#[cfg(feature = "testing")]
impl TryFrom<RequestData> for previous::interface::RequestData {
    type Error = super::Error;

    fn try_from(value: RequestData) -> Result<Self, Self::Error> {
        match value {
            RequestData::AvailableBandwidth(inner) => Ok(Self::AvailableBandwidth(inner)),
            RequestData::TopUpBandwidth(_) => Ok(Self::TopUpBandwidth(())),
        }
    }
}

// this should be with #[cfg(feature = "testing")] only coming from v0, don't copy this for future versions
#[cfg(feature = "testing")]
impl TryFrom<previous::interface::ResponseData> for ResponseData {
    type Error = super::Error;

    fn try_from(value: previous::interface::ResponseData) -> Result<Self, Self::Error> {
        match value {
            previous::interface::ResponseData::AvailableBandwidth(_) => {
                Err(super::Error::UpdateNotPossible {
                    from: previous::VERSION,
                    to: VERSION,
                })
            }
            previous::interface::ResponseData::TopUpBandwidth(_) => {
                Err(super::Error::UpdateNotPossible {
                    from: previous::VERSION,
                    to: VERSION,
                })
            }
        }
    }
}

// this should be with #[cfg(feature = "testing")] only coming from v0, don't copy this for future versions
#[cfg(feature = "testing")]
impl TryFrom<ResponseData> for previous::interface::ResponseData {
    type Error = super::Error;

    fn try_from(value: ResponseData) -> Result<Self, Self::Error> {
        match value {
            ResponseData::AvailableBandwidth(_) => Ok(Self::AvailableBandwidth(())),
            ResponseData::TopUpBandwidth(_) => Ok(Self::TopUpBandwidth(())),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::models::tests::CREDENTIAL_BYTES;

    use super::*;

    #[test]
    fn request_upgrade() {
        assert_eq!(
            RequestData::try_from(previous::interface::RequestData::AvailableBandwidth(()))
                .unwrap(),
            RequestData::AvailableBandwidth(())
        );
        assert!(
            RequestData::try_from(previous::interface::RequestData::TopUpBandwidth(())).is_err(),
        );
    }

    #[test]
    fn response_upgrade() {
        assert!(
            ResponseData::try_from(previous::interface::ResponseData::AvailableBandwidth(()))
                .is_err()
        );
        assert!(
            ResponseData::try_from(previous::interface::ResponseData::TopUpBandwidth(())).is_err()
        );
    }

    #[test]
    fn request_downgrade() {
        assert_eq!(
            previous::interface::RequestData::try_from(RequestData::AvailableBandwidth(()))
                .unwrap(),
            previous::interface::RequestData::AvailableBandwidth(())
        );
        assert_eq!(
            previous::interface::RequestData::try_from(RequestData::TopUpBandwidth(Box::new(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap()
            )))
            .unwrap(),
            previous::interface::RequestData::TopUpBandwidth(())
        );
    }

    #[test]
    fn response_downgrade() {
        assert_eq!(
            previous::interface::ResponseData::try_from(ResponseData::AvailableBandwidth(42))
                .unwrap(),
            previous::interface::ResponseData::AvailableBandwidth(())
        );
        assert_eq!(
            previous::interface::ResponseData::try_from(ResponseData::TopUpBandwidth(42)).unwrap(),
            previous::interface::ResponseData::TopUpBandwidth(())
        );
    }
}
