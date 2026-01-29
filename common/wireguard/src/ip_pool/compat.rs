// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ip_pool::IpPair;

impl From<IpPair> for nym_authenticator_requests::v6::registration::IpPair {
    fn from(ip_pair: IpPair) -> Self {
        nym_authenticator_requests::v6::registration::IpPair {
            ipv4: ip_pair.ipv4,
            ipv6: ip_pair.ipv6,
        }
    }
}

impl From<IpPair> for nym_authenticator_requests::v5::registration::IpPair {
    fn from(ip_pair: IpPair) -> Self {
        nym_authenticator_requests::v5::registration::IpPair {
            ipv4: ip_pair.ipv4,
            ipv6: ip_pair.ipv6,
        }
    }
}

impl From<IpPair> for nym_authenticator_requests::v4::registration::IpPair {
    fn from(ip_pair: IpPair) -> Self {
        nym_authenticator_requests::v4::registration::IpPair {
            ipv4: ip_pair.ipv4,
            ipv6: ip_pair.ipv6,
        }
    }
}

//

impl From<nym_authenticator_requests::v6::registration::IpPair> for IpPair {
    fn from(ip_pair: nym_authenticator_requests::v6::registration::IpPair) -> Self {
        IpPair {
            ipv4: ip_pair.ipv4,
            ipv6: ip_pair.ipv6,
        }
    }
}

impl From<nym_authenticator_requests::v5::registration::IpPair> for IpPair {
    fn from(ip_pair: nym_authenticator_requests::v5::registration::IpPair) -> Self {
        IpPair {
            ipv4: ip_pair.ipv4,
            ipv6: ip_pair.ipv6,
        }
    }
}

impl From<nym_authenticator_requests::v4::registration::IpPair> for IpPair {
    fn from(ip_pair: nym_authenticator_requests::v4::registration::IpPair) -> Self {
        IpPair {
            ipv4: ip_pair.ipv4,
            ipv6: ip_pair.ipv6,
        }
    }
}
