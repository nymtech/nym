// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{v1, v2, v3, v4, v5, v6};
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

#[derive(Copy, Clone, Debug, PartialEq, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum AuthenticatorVersion {
    /// introduced in wispa release (1.1.5)
    V1,

    /// introduced in aero release (1.1.9)
    V2,

    /// introduced in magura release (1.1.10)
    V3,

    /// introduced in crunch release (1.2.0)
    V4,

    /// introduced in dorina-patched release (1.6.1)
    V5,

    /// introduced in niolo release (1.23.0)
    V6,

    /// an unknown, future, variant that can be present if running outdated software
    UNKNOWN,
}

impl AuthenticatorVersion {
    pub const LATEST: Self = Self::V6;

    pub const fn release_version(&self) -> semver::Version {
        match self {
            AuthenticatorVersion::V1 => semver::Version::new(1, 1, 5),
            AuthenticatorVersion::V2 => semver::Version::new(1, 1, 9),
            AuthenticatorVersion::V3 => semver::Version::new(1, 1, 10),
            AuthenticatorVersion::V4 => semver::Version::new(1, 2, 0),
            AuthenticatorVersion::V5 => semver::Version::new(1, 6, 1),
            AuthenticatorVersion::V6 => semver::Version::new(1, 23, 0),
            AuthenticatorVersion::UNKNOWN => semver::Version::new(0, 0, 0),
        }
    }
}

impl From<Protocol> for AuthenticatorVersion {
    fn from(value: Protocol) -> Self {
        if value.service_provider_type != ServiceProviderType::Authenticator {
            AuthenticatorVersion::UNKNOWN
        } else if value.version == v1::VERSION {
            AuthenticatorVersion::V1
        } else if value.version == v2::VERSION {
            AuthenticatorVersion::V2
        } else if value.version == v3::VERSION {
            AuthenticatorVersion::V3
        } else if value.version == v4::VERSION {
            AuthenticatorVersion::V4
        } else if value.version == v5::VERSION {
            AuthenticatorVersion::V5
        } else if value.version == v6::VERSION {
            AuthenticatorVersion::V6
        } else {
            AuthenticatorVersion::UNKNOWN
        }
    }
}

impl From<u8> for AuthenticatorVersion {
    fn from(value: u8) -> Self {
        if value == v1::VERSION {
            AuthenticatorVersion::V1
        } else if value == v2::VERSION {
            AuthenticatorVersion::V2
        } else if value == v3::VERSION {
            AuthenticatorVersion::V3
        } else if value == v4::VERSION {
            AuthenticatorVersion::V4
        } else if value == v5::VERSION {
            AuthenticatorVersion::V5
        } else if value == v6::VERSION {
            AuthenticatorVersion::V6
        } else {
            AuthenticatorVersion::UNKNOWN
        }
    }
}

impl From<&str> for AuthenticatorVersion {
    fn from(value: &str) -> Self {
        let Ok(semver) = semver::Version::parse(value) else {
            return Self::UNKNOWN;
        };

        semver.into()
    }
}

impl From<Option<&String>> for AuthenticatorVersion {
    fn from(value: Option<&String>) -> Self {
        match value {
            None => Self::UNKNOWN,
            Some(value) => value.as_str().into(),
        }
    }
}

impl From<String> for AuthenticatorVersion {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<Option<String>> for AuthenticatorVersion {
    fn from(value: Option<String>) -> Self {
        value.as_ref().into()
    }
}

impl From<semver::Version> for AuthenticatorVersion {
    fn from(semver: semver::Version) -> Self {
        if semver < AuthenticatorVersion::V1.release_version() {
            return Self::UNKNOWN;
        }
        if semver < AuthenticatorVersion::V2.release_version() {
            return Self::V1;
        }
        if semver < AuthenticatorVersion::V3.release_version() {
            return Self::V2;
        }
        if semver < AuthenticatorVersion::V4.release_version() {
            return Self::V3;
        }
        if semver < AuthenticatorVersion::V5.release_version() {
            return Self::V4;
        }
        if semver < AuthenticatorVersion::V6.release_version() {
            return Self::V5;
        }
        // if provided version is higher (or equal) to release version of V6,
        // we return the latest (i.e. v6)

        debug_assert_eq!(
            Self::V6,
            Self::LATEST,
            "a new AuthenticatorVersion variant has been introduced without adjusting the `From<semver::Version>` trait"
        );
        Self::LATEST
    }
}

#[cfg(test)]
mod tests {
    use super::super::latest;
    use super::*;

    #[test]
    fn strum_display() {
        // sanity check on formatting and casing
        assert_eq!("v1", AuthenticatorVersion::V1.to_string());
        assert_eq!("v2", AuthenticatorVersion::V2.to_string());
        assert_eq!("unknown", AuthenticatorVersion::UNKNOWN.to_string());
    }

    #[test]
    fn u8_conversion() {
        assert_eq!(AuthenticatorVersion::V1, AuthenticatorVersion::from(1u8));
        assert_eq!(AuthenticatorVersion::V2, AuthenticatorVersion::from(2u8));

        assert_eq!(
            AuthenticatorVersion::UNKNOWN,
            AuthenticatorVersion::from(latest::VERSION + 1)
        );
        assert_eq!(
            AuthenticatorVersion::UNKNOWN,
            AuthenticatorVersion::from(0u8)
        );
        assert_eq!(
            AuthenticatorVersion::UNKNOWN,
            AuthenticatorVersion::from(255u8)
        );
    }

    #[test]
    fn semver_checks() {
        assert_eq!(AuthenticatorVersion::UNKNOWN, "1.1.4".into());
        assert_eq!(AuthenticatorVersion::UNKNOWN, "0.1.0".into());
        assert_eq!(AuthenticatorVersion::UNKNOWN, "1.0.4".into());
        assert_eq!(AuthenticatorVersion::V1, "1.1.5".into());
        assert_eq!(AuthenticatorVersion::V1, "1.1.6".into());
        assert_eq!(AuthenticatorVersion::V1, "1.1.8".into());
        assert_eq!(AuthenticatorVersion::V2, "1.1.9".into());
        assert_eq!(AuthenticatorVersion::V3, "1.1.10".into());
        assert_eq!(AuthenticatorVersion::V3, "1.1.11".into());
        assert_eq!(AuthenticatorVersion::V3, "1.1.60".into());
        assert_eq!(AuthenticatorVersion::V4, "1.2.0".into());
        assert_eq!(AuthenticatorVersion::V4, "1.2.1".into());
        assert_eq!(AuthenticatorVersion::V4, "1.5.1".into());
        assert_eq!(AuthenticatorVersion::V4, "1.6.0".into());
        assert_eq!(AuthenticatorVersion::V5, "1.6.1".into());
        assert_eq!(AuthenticatorVersion::V5, "1.6.11".into());
        assert_eq!(AuthenticatorVersion::V5, "1.7.0".into());
        assert_eq!(AuthenticatorVersion::V5, "1.16.11".into());
        assert_eq!(AuthenticatorVersion::V5, "1.17.0".into());
        assert_eq!(AuthenticatorVersion::V5, "1.22.0".into());
        assert_eq!(AuthenticatorVersion::V6, "1.23.0".into());
        assert_eq!(AuthenticatorVersion::V6, "1.23.1".into());
        assert_eq!(AuthenticatorVersion::V6, "1.24.0".into());
    }
}
