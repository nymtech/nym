// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::{EmptyMessage, ServiceProviderRequest};

/// Defines initial version of the communication interface between clients and service providers.
// note: we start from '3' so that we could distinguish cases where no version is provided
// and legacy communication mode is used instead
pub const INITIAL_INTERFACE_VERSION: u8 = 3;

/// Defines the current version of the communication interface between clients and service providers.
/// It has to be incremented for any breaking change.
pub const INTERFACE_VERSION: u8 = 3;

/// Defines full version of particular request that includes version of common service provider interface
/// and provider-specific protocol.
#[derive(Debug, Clone)]
pub struct RequestVersion<T: ServiceProviderRequest = EmptyMessage> {
    /// Defines version used for the interface shared by all service providers,
    /// such as available control messages and their serialization.
    pub provider_interface: ProviderInterfaceVersion,

    /// Defines version used specifically by particular provider's protocol.
    /// For example, it could be the socks5 protocol used by socks5 client
    /// and the network requester.
    pub provider_protocol: T::ProtocolVersion,
}

impl<T: ServiceProviderRequest> RequestVersion<T> {
    pub fn new(
        provider_interface: ProviderInterfaceVersion,
        provider_protocol: T::ProtocolVersion,
    ) -> Self {
        RequestVersion {
            provider_interface,
            provider_protocol,
        }
    }
}

pub trait Version: std::fmt::Display {}

#[macro_export]
macro_rules! define_simple_version {
    ($name: ident, $initial_version: ident, $current_version: ident) => {
        use serde::{Deserialize, Serialize};
        use std::fmt::{self, Display, Formatter};

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
        #[serde(tag = "type", content = "version")]
        pub enum $name {
            Legacy,
            Versioned(u8),
        }

        impl $name {
            pub const fn new(use_legacy: bool) -> Self {
                if use_legacy {
                    Self::new_legacy()
                } else {
                    Self::new_versioned($current_version)
                }
            }

            pub const fn new_legacy() -> Self {
                $name::Legacy
            }

            pub const fn new_versioned(version: u8) -> Self {
                $name::Versioned(version)
            }

            pub const fn new_current() -> Self {
                $name::new(false)
            }

            pub const fn is_legacy(&self) -> bool {
                matches!(self, $name::Legacy)
            }

            pub const fn as_u8(&self) -> Option<u8> {
                match self {
                    $name::Legacy => None,
                    $name::Versioned(version) => Some(*version),
                }
            }
        }

        impl From<u8> for $name {
            fn from(v: u8) -> Self {
                match v {
                    n if n < $initial_version => $name::Legacy,
                    n => $name::Versioned(n),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name::Versioned(INTERFACE_VERSION)
            }
        }

        // I'm not fully convinced about this just yet...
        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}", serde_json::to_string(&self).unwrap())
            }
        }

        impl Version for $name {}
    };
}

define_simple_version!(
    ProviderInterfaceVersion,
    INITIAL_INTERFACE_VERSION,
    INTERFACE_VERSION
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_version_ordering() {
        // in case something is done to the original enum/macro, make sure the below assumptions still hold
        assert!(ProviderInterfaceVersion::Legacy < ProviderInterfaceVersion::Versioned(0));
        assert!(ProviderInterfaceVersion::Legacy < ProviderInterfaceVersion::Versioned(1));
        assert!(ProviderInterfaceVersion::Versioned(1) < ProviderInterfaceVersion::Versioned(2));
        assert!(ProviderInterfaceVersion::Versioned(42) < ProviderInterfaceVersion::Versioned(100));
    }
}
