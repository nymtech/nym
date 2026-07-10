// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Gateway selection and construction of the per-hop `RegistrationNymNode` the
//! LP registration client consumes.

use std::net::SocketAddr;
use std::sync::Arc;

use nym_api_requests::models::described::v2::NymNodeDescriptionV2;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_registration_client::RegistrationNymNode;
use nym_registration_common::{NymNodeInformation, NymNodeLPInformation};
use rand::seq::SliceRandom;

use crate::dvpn::{DvpnDirectory, QuicBridge};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// The gateway's directory node id.
    pub node_id: u32,
    /// The gateway's advertised IP address.
    pub ip: std::net::IpAddr,
    /// The gateway's human moniker from the dVPN directory, if configured/known.
    pub name: Option<String>,
    /// The gateway's QUIC bridge parameters, if it advertises one.
    pub quic: Option<QuicBridge>,
}

impl SelectedGateway {
    /// A copyable summary of this gateway's directory metadata.
    pub fn info(&self) -> GatewayInfo {
        GatewayInfo {
            identity: self.identity,
            node_id: self.node_id,
            country: self.country.clone(),
            ip: self.ip,
            name: self.name.clone(),
        }
    }
}

/// Directory metadata for the gateway a tunnel hop terminates at.
#[derive(Clone, Debug)]
pub struct GatewayInfo {
    /// ed25519 identity key.
    pub identity: ed25519::PublicKey,
    /// Directory node id.
    pub node_id: u32,
    /// Advertised country (ISO 3166 alpha-2), if known.
    pub country: Option<String>,
    /// Advertised IP address.
    pub ip: std::net::IpAddr,
    /// Human moniker from the dVPN directory, if configured/known.
    pub name: Option<String>,
}

/// A node is usable for dVPN only if it advertises WireGuard, an authenticator, and LP data.
///
/// The node's declared mixnet entry/exit role is deliberately NOT checked: dVPN does not
/// distinguish entry from exit nodes (that role only constrains mixnet mode), so any
/// WireGuard-capable node can serve either dVPN hop. `role` is retained for API symmetry and future
/// use but no longer filters the candidate set.
fn wg_capable(desc: &NymNodeDescriptionV2, _role: WgRole) -> bool {
    let d = &desc.description;
    d.wireguard.is_some() && d.lewes_protocol.is_some() && d.authenticator.is_some()
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
        node_id: desc.node_id,
        ip,
        name: None,
        quic: None,
    })
}

/// Build a gateway and enrich its moniker/QUIC bridge from the dVPN directory.
fn build_and_enrich(
    desc: &NymNodeDescriptionV2,
    directory: Option<&DvpnDirectory>,
) -> Result<SelectedGateway, SessionError> {
    let mut selected = build_node(desc)?;
    if let Some(entry) = directory.and_then(|d| d.entry(&selected.identity.to_base58_string())) {
        selected.name = entry.name.clone();
        selected.quic = entry.quic.clone();
        // Prefer the directory's country when the described node lacks one.
        if selected.country.is_none() {
            selected.country = entry.country.clone();
        }
    }
    Ok(selected)
}

/// Whether `identity` may be selected given the QUIC requirement.
fn quic_ok(
    directory: Option<&DvpnDirectory>,
    require_quic: bool,
    identity: &ed25519::PublicKey,
) -> bool {
    !require_quic || directory.is_some_and(|d| d.has_quic(&identity.to_base58_string()))
}

/// Select a gateway from the described-node set per the spec and role.
///
/// When `require_quic` is set, only gateways the dVPN `directory` reports as
/// QUIC-bridge-capable are eligible; if none match, [`SessionError::NoQuicGateway`]
/// is returned. `exclude` (the already-chosen hop's identity, e.g. the entry when
/// picking the exit) is never selected, so a two-hop tunnel gets distinct gateways.
pub(crate) fn select(
    nodes: &[NymNodeDescriptionV2],
    spec: &GatewaySpec,
    role: WgRole,
    directory: Option<&DvpnDirectory>,
    require_quic: bool,
    exclude: Option<&ed25519::PublicKey>,
) -> Result<SelectedGateway, SessionError> {
    let excluded = |id: &ed25519::PublicKey| exclude == Some(id);
    match spec {
        GatewaySpec::Identity(id) => {
            if excluded(id) {
                return Err(SessionError::SameGatewaySelected(id.to_base58_string()));
            }
            let desc = nodes
                .iter()
                .find(|n| &n.ed25519_identity_key() == id)
                .ok_or_else(|| SessionError::GatewayNotFound(id.to_base58_string()))?;
            if !wg_capable(desc, role) {
                return Err(SessionError::NoWireguardGateway);
            }
            if !quic_ok(directory, require_quic, id) {
                return Err(SessionError::NoQuicGateway {
                    spec: id.to_base58_string(),
                });
            }
            build_and_enrich(desc, directory)
        }
        GatewaySpec::Country(cc) => {
            let candidates: Vec<&NymNodeDescriptionV2> = nodes
                .iter()
                .filter(|n| {
                    let id = n.ed25519_identity_key();
                    !excluded(&id)
                        && wg_capable(n, role)
                        && n.description
                            .auxiliary_details
                            .location
                            .as_ref()
                            .is_some_and(|c| c.alpha2.eq_ignore_ascii_case(cc))
                        && quic_ok(directory, require_quic, &id)
                })
                .collect();
            let desc = candidates.choose(&mut rand::thread_rng()).ok_or_else(|| {
                if require_quic {
                    SessionError::NoQuicGateway {
                        spec: format!("country {cc}"),
                    }
                } else {
                    SessionError::NoCountryMatch(cc.clone())
                }
            })?;
            build_and_enrich(desc, directory)
        }
        GatewaySpec::Random => {
            let candidates: Vec<&NymNodeDescriptionV2> = nodes
                .iter()
                .filter(|n| {
                    let id = n.ed25519_identity_key();
                    !excluded(&id) && wg_capable(n, role) && quic_ok(directory, require_quic, &id)
                })
                .collect();
            let desc = candidates.choose(&mut rand::thread_rng()).ok_or_else(|| {
                if require_quic {
                    SessionError::NoQuicGateway {
                        spec: "random".to_string(),
                    }
                } else {
                    SessionError::NoWireguardGateway
                }
            })?;
            build_and_enrich(desc, directory)
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
        let err = select(
            &[],
            &GatewaySpec::Identity(id),
            WgRole::Entry,
            None,
            false,
            None,
        )
        .err()
        .expect("expected selection error");
        match err {
            SessionError::GatewayNotFound(s) => assert_eq!(s, id.to_base58_string()),
            other => panic!("expected GatewayNotFound, got {other:?}"),
        }
    }

    #[test]
    fn excluded_identity_is_rejected() {
        // Selecting the excluded gateway (e.g. the entry, when picking the exit)
        // fails up front so a two-hop tunnel gets distinct gateways.
        let id =
            ed25519::PublicKey::from_base58_string("Gejc2CnSRFUxK6519ewmWM66ytDZbbuXytwLUgytCQUD")
                .unwrap();
        let err = select(
            &[],
            &GatewaySpec::Identity(id),
            WgRole::Exit,
            None,
            false,
            Some(&id),
        )
        .err()
        .expect("expected selection error");
        match err {
            SessionError::SameGatewaySelected(s) => assert_eq!(s, id.to_base58_string()),
            other => panic!("expected SameGatewaySelected, got {other:?}"),
        }
    }

    #[test]
    fn country_no_match_over_empty_set() {
        let err = select(
            &[],
            &GatewaySpec::Country("CH".into()),
            WgRole::Exit,
            None,
            false,
            None,
        )
        .err()
        .expect("expected selection error");
        match err {
            SessionError::NoCountryMatch(cc) => assert_eq!(cc, "CH"),
            other => panic!("expected NoCountryMatch, got {other:?}"),
        }
    }

    #[test]
    fn random_no_gateway_over_empty_set() {
        let err = select(&[], &GatewaySpec::Random, WgRole::Entry, None, false, None)
            .err()
            .expect("expected selection error");
        assert!(matches!(err, SessionError::NoWireguardGateway));
    }

    #[test]
    fn require_quic_without_directory_fails() {
        // With no directory (None), requiring QUIC can never be satisfied.
        let err = select(&[], &GatewaySpec::Random, WgRole::Entry, None, true, None)
            .err()
            .expect("expected selection error");
        assert!(matches!(err, SessionError::NoQuicGateway { .. }));
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
