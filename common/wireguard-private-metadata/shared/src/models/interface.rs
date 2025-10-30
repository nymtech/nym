// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::BandwidthCredential;

#[cfg(feature = "testing")]
use crate::models::v0;
use crate::models::{Construct, Extract, Request, Response, Version, v1, v2};

pub enum RequestData {
    AvailableBandwidth,
    TopUpBandwidth {
        credential: Box<BandwidthCredential>,
    },
}

impl From<super::latest::interface::RequestData> for RequestData {
    fn from(value: super::latest::interface::RequestData) -> Self {
        match value {
            super::latest::interface::RequestData::AvailableBandwidth => Self::AvailableBandwidth,
            super::latest::interface::RequestData::TopUpBandwidth { credential } => {
                Self::TopUpBandwidth { credential }
            }
        }
    }
}

impl From<RequestData> for super::latest::interface::RequestData {
    fn from(value: RequestData) -> Self {
        match value {
            RequestData::AvailableBandwidth => Self::AvailableBandwidth,
            RequestData::TopUpBandwidth { credential } => Self::TopUpBandwidth { credential },
        }
    }
}

impl From<super::latest::interface::ResponseData> for ResponseData {
    fn from(value: super::latest::interface::ResponseData) -> Self {
        match value {
            super::latest::interface::ResponseData::AvailableBandwidth {
                amount,
                upgrade_mode,
            } => Self::AvailableBandwidth {
                amount,
                upgrade_mode,
            },
            super::latest::interface::ResponseData::TopUpBandwidth {
                available_bandwidth,
                upgrade_mode,
            } => Self::TopUpBandwidth {
                available_bandwidth,
                upgrade_mode,
            },
        }
    }
}

impl From<ResponseData> for super::latest::interface::ResponseData {
    fn from(value: ResponseData) -> Self {
        match value {
            ResponseData::AvailableBandwidth {
                amount,
                upgrade_mode,
            } => Self::AvailableBandwidth {
                amount,
                upgrade_mode,
            },
            ResponseData::TopUpBandwidth {
                available_bandwidth,
                upgrade_mode,
            } => Self::TopUpBandwidth {
                available_bandwidth,
                upgrade_mode,
            },
        }
    }
}

impl Construct<RequestData> for Request {
    fn construct(info: RequestData, version: Version) -> Result<Self, super::error::Error> {
        match version {
            #[cfg(feature = "testing")]
            Version::V0 => {
                // attempt to go through conversion chain for `info`: v2 => v1 => v0
                let v2_info = v2::interface::RequestData::from(info);
                let v1_info = v1::interface::RequestData::try_from(v2_info)?;
                let v0_info = v0::interface::RequestData::try_from(v1_info)?;

                let versioned_request = v0::VersionedRequest::construct(v0_info, version)?;
                Ok(versioned_request.try_into()?)
            }
            Version::V1 => {
                // attempt to go through conversion chain for `info`: v2 => v1
                let v2_info = v2::interface::RequestData::from(info);
                let v1_info = v1::interface::RequestData::try_from(v2_info)?;

                let versioned_request = v1::VersionedRequest::construct(v1_info, version)?;
                Ok(versioned_request.try_into()?)
            }
            Version::V2 => {
                let v2_info = v2::interface::RequestData::from(info);

                let versioned_request = v2::VersionedRequest::construct(v2_info, version)?;
                Ok(versioned_request.try_into()?)
            }
        }
    }
}

impl Extract<RequestData> for Request {
    fn extract(&self) -> Result<(RequestData, Version), crate::models::Error> {
        match self.version {
            #[cfg(feature = "testing")]
            super::Version::V0 => {
                let versioned_request = v0::VersionedRequest::try_from(self.clone())?;
                let (extracted_v0_info, version) = versioned_request.extract()?;

                let v1_info = v1::interface::RequestData::try_from(extracted_v0_info)?;
                let v2_info = v2::interface::RequestData::try_from(v1_info)?;

                let request_data = RequestData::from(v2_info);
                Ok((request_data, version))
            }
            super::Version::V1 => {
                let versioned_request = v1::VersionedRequest::try_from(self)?;
                let (extracted_v1_info, version) = versioned_request.extract()?;
                let v2_info = v2::interface::RequestData::try_from(extracted_v1_info)?;

                let request_data = RequestData::from(v2_info);
                Ok((request_data, version))
            }
            super::Version::V2 => {
                let versioned_request = v2::VersionedRequest::try_from(self)?;
                let (extracted_v2_info, version) = versioned_request.extract()?;

                let request_data = RequestData::from(extracted_v2_info);
                Ok((request_data, version))
            }
        }
    }
}

pub enum ResponseData {
    AvailableBandwidth {
        amount: i64,
        upgrade_mode: bool,
    },
    TopUpBandwidth {
        available_bandwidth: i64,
        upgrade_mode: bool,
    },
}

impl Construct<ResponseData> for Response {
    fn construct(info: ResponseData, version: Version) -> Result<Self, super::error::Error> {
        match version {
            #[cfg(feature = "testing")]
            super::Version::V0 => {
                // attempt to go through conversion chain for `info`: v2 => v1 => v0
                let v2_info = v2::interface::ResponseData::from(info);
                let v1_info = v1::interface::ResponseData::try_from(v2_info)?;
                let v0_info = v0::interface::ResponseData::try_from(v1_info)?;

                let versioned_response = v0::VersionedResponse::construct(v0_info, version)?;
                Ok(versioned_response.try_into()?)
            }
            Version::V1 => {
                // attempt to go through conversion chain for `info`: v2 => v1
                let v2_info = v2::interface::ResponseData::from(info);
                let v1_info = v1::interface::ResponseData::try_from(v2_info)?;

                let versioned_response = v1::VersionedResponse::construct(v1_info, version)?;
                Ok(versioned_response.try_into()?)
            }
            Version::V2 => {
                let v2_info = v2::interface::ResponseData::from(info);

                let versioned_response = v2::VersionedResponse::construct(v2_info, version)?;
                Ok(versioned_response.try_into()?)
            }
        }
    }
}

impl Extract<ResponseData> for Response {
    fn extract(&self) -> Result<(ResponseData, Version), super::error::Error> {
        match self.version {
            #[cfg(feature = "testing")]
            super::Version::V0 => {
                let versioned_response = v0::VersionedResponse::try_from(self.clone())?;
                let (extracted_v0_info, version) = versioned_response.extract()?;
                let v1_info = v1::interface::ResponseData::try_from(extracted_v0_info)?;
                let v2_info = v2::interface::ResponseData::try_from(v1_info)?;

                let response_data = ResponseData::from(v2_info);
                Ok((response_data, version))
            }
            super::Version::V1 => {
                let versioned_response = v1::VersionedResponse::try_from(self.clone())?;
                let (extracted_v1_info, version) = versioned_response.extract()?;
                let v2_info = v2::interface::ResponseData::try_from(extracted_v1_info)?;

                let response_data = ResponseData::from(v2_info);
                Ok((response_data, version))
            }
            super::Version::V2 => {
                let versioned_response = v2::VersionedResponse::try_from(self.clone())?;
                let (extracted_v2_info, version) = versioned_response.extract()?;

                let response_data = ResponseData::from(extracted_v2_info);
                Ok((response_data, version))
            }
        }
    }
}
