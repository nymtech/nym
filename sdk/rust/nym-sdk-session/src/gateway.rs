// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Gateway selection and construction of the per-hop `RegistrationNymNode` the
//! LP registration client consumes.

use std::net::SocketAddr;
use std::sync::Arc;

use nym_api_requests::models::described::v2::NymNodeDescriptionV2;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_registration_client::RegistrationNymNode;
use nym_registration_common::{NymNodeInformation, NymNodeLPInformation};
use rand::seq::SliceRandom;

use crate::error::SessionError;

/// How the caller names the gateway(s) to use.
#[derive(Clone, Debug)]
pub enum GatewaySpec {
    /// An exact gateway identity (ed25519) key.
    Identity(ed25519::PublicKey),
    /// A two-letter ISO 3166 alpha-2 country code; a random match is chosen.
    Country(String),
    /// A uniformly random WireGuard-capable gateway.
    Random,
}

/// Which WireGuard role a gateway must fulfil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WgRole {
    /// Entry gateway (must declare the `entry` role).
    Entry,
    /// Exit gateway (must be able to operate as an exit gateway).
    Exit,
}

/// A selected, registration-ready gateway.
pub struct SelectedGateway {
    /// The node plus a freshly generated client WireGuard keypair.
    pub node: RegistrationNymNode,
    /// The gateway's ed25519 identity.
    pub identity: ed25519::PublicKey,
    /// The gateway's advertised country (ISO 3166 alpha-2), if known.
    pub country: Option<String>,
}

/// A node is usable for dVPN only if it advertises WireGuard, an authenticator,
/// and LP data, and fulfils the required role.
fn wg_capable(desc: &NymNodeDescriptionV2, role: WgRole) -> bool {
    let d = &desc.description;
    if d.wireguard.is_none() || d.lewes_protocol.is_none() || d.authenticator.is_none() {
        return false;
    }
    match role {
        WgRole::Entry => d.declared_role.entry,
        WgRole::Exit => d.declared_role.can_operate_exit_gateway(),
    }
}

/// Build the LP information block for a node, verifying its LP signature and
/// deriving the ciphersuite from the node's version.
fn build_lp(
    desc: &NymNodeDescriptionV2,
    identity: &ed25519::PublicKey,
    ip: std::net::IpAddr,
) -> Result<Option<NymNodeLPInformation>, SessionError> {
    let Some(lp) = desc.description.lewes_protocol.as_ref() else {
        return Ok(None);
    };

    let malformed = |reason: &str| SessionError::MalformedGateway {
        identity: identity.to_base58_string(),
        reason: reason.to_string(),
    };

    if !lp.verify(identity) {
        return Err(malformed("invalid LP signature"));
    }

    let version: semver::Version = desc
        .description
        .build_information
        .build_version
        .parse()
        .map_err(|_| malformed("unparseable build version"))?;
    let ciphersuite = nym_kkt_ciphersuite::Ciphersuite::from_node_version(version)
        .ok_or_else(|| malformed("no valid ciphersuite for node version"))?;
    let expected_kem_key_hashes = lp
        .content
        .kem_keys()
        .map_err(|_| malformed("malformed LP KEM key digests"))?;

    Ok(Some(NymNodeLPInformation {
        address: SocketAddr::new(ip, lp.content.control_port),
        expected_kem_key_hashes,
        x25519: lp.content.x25519,
        ciphersuite,
        // The directory carries no per-node LP protocol version; use ours.
        lp_protocol_version: nym_lp_data::packet::version::CURRENT,
    }))
}

/// Construct a `RegistrationNymNode` (node info + a fresh client WG keypair)
/// from a described node.
fn build_node(desc: &NymNodeDescriptionV2) -> Result<SelectedGateway, SessionError> {
    let identity = desc.ed25519_identity_key();
    let ip = desc
        .description
        .host_information
        .ip_address
        .first()
        .copied()
        .ok_or_else(|| SessionError::MalformedGateway {
            identity: identity.to_base58_string(),
            reason: "no advertised IP address".to_string(),
        })?;

    let lp_data = build_lp(desc, &identity, ip)?;
    let authenticator_address = desc
        .description
        .authenticator
        .as_ref()
        .and_then(|a| a.address.parse().ok());
    let version = nym_authenticator_requests::AuthenticatorVersion::from(
        desc.description.build_information.build_version.as_str(),
    );
    let country = desc
        .description
        .auxiliary_details
        .location
        .as_ref()
        .map(|c| c.alpha2.to_string());

    let node = NymNodeInformation {
        identity,
        ip_address: ip,
        ipr_address: None,
        authenticator_address,
        lp_data,
        version,
    };
    // Fresh per-hop client WireGuard keypair.
    let keys = Arc::new(x25519::KeyPair::new(&mut rand::thread_rng()));

    Ok(SelectedGateway {
        node: RegistrationNymNode { node, keys },
        identity,
        country,
    })
}

/// Select a gateway from the described-node set per the spec and role.
pub(crate) fn select(
    nodes: Vec<NymNodeDescriptionV2>,
    spec: &GatewaySpec,
    role: WgRole,
) -> Result<SelectedGateway, SessionError> {
    match spec {
        GatewaySpec::Identity(id) => {
            let desc = nodes
                .into_iter()
                .find(|n| &n.ed25519_identity_key() == id)
                .ok_or_else(|| SessionError::GatewayNotFound(id.to_base58_string()))?;
            if !wg_capable(&desc, role) {
                return Err(SessionError::NoWireguardGateway);
            }
            build_node(&desc)
        }
        GatewaySpec::Country(cc) => {
            let candidates: Vec<_> = nodes
                .into_iter()
                .filter(|n| {
                    wg_capable(n, role)
                        && n.description
                            .auxiliary_details
                            .location
                            .as_ref()
                            .is_some_and(|c| c.alpha2.eq_ignore_ascii_case(cc))
                })
                .collect();
            let desc = candidates
                .choose(&mut rand::thread_rng())
                .ok_or_else(|| SessionError::NoCountryMatch(cc.clone()))?;
            build_node(desc)
        }
        GatewaySpec::Random => {
            let candidates: Vec<_> = nodes.into_iter().filter(|n| wg_capable(n, role)).collect();
            let desc = candidates
                .choose(&mut rand::thread_rng())
                .ok_or(SessionError::NoWireguardGateway)?;
            build_node(desc)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Selection error-path + role unit tests (OpenSpec task 3.8). Constructing
    //! a fully-valid `NymNodeDescriptionV2` set is impractical, so these cover
    //! the selection logic over an empty candidate set (the not-found / no-match
    //! branches for every `GatewaySpec`), plus error surfacing — the paths a
    //! caller depends on when a gateway is missing or unsupported.

    use super::*;

    fn random_identity() -> ed25519::PublicKey {
        *ed25519::KeyPair::new(&mut rand::thread_rng()).public_key()
    }

    #[test]
    fn identity_not_found_over_empty_set() {
        let id = random_identity();
        let err = select(vec![], &GatewaySpec::Identity(id), WgRole::Entry)
            .err()
            .expect("expected selection error");
        match err {
            SessionError::GatewayNotFound(s) => assert_eq!(s, id.to_base58_string()),
            other => panic!("expected GatewayNotFound, got {other:?}"),
        }
    }

    #[test]
    fn country_no_match_over_empty_set() {
        let err = select(vec![], &GatewaySpec::Country("CH".into()), WgRole::Exit)
            .err()
            .expect("expected selection error");
        match err {
            SessionError::NoCountryMatch(cc) => assert_eq!(cc, "CH"),
            other => panic!("expected NoCountryMatch, got {other:?}"),
        }
    }

    #[test]
    fn random_no_gateway_over_empty_set() {
        let err = select(vec![], &GatewaySpec::Random, WgRole::Entry)
            .err()
            .expect("expected selection error");
        assert!(matches!(err, SessionError::NoWireguardGateway));
    }

    #[test]
    fn role_is_copy_and_comparable() {
        let r = WgRole::Entry;
        let r2 = r; // Copy
        assert_eq!(r, r2);
        assert_ne!(WgRole::Entry, WgRole::Exit);
    }

    #[test]
    fn error_messages_are_descriptive() {
        assert!(SessionError::NoWireguardGateway
            .to_string()
            .contains("WireGuard-capable"));
        assert!(SessionError::NoCountryMatch("DE".into())
            .to_string()
            .contains("DE"));
        assert_eq!(
            SessionError::Cancelled.to_string(),
            "session setup was cancelled"
        );
    }
}
