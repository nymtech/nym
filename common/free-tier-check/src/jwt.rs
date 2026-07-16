// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::FreeTierCheckError;
use jwt_simple::claims::Claims;
use jwt_simple::common::{KeyMetadata, VerificationOptions};
use jwt_simple::prelude::{EdDSAKeyPairLike, EdDSAPublicKeyLike};
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Issuer identity of the Nym credential proxy. Mirrors the constant in
/// `nym-upgrade-mode-check`; both features verify credential-proxy-signed JWTs.
pub const CREDENTIAL_PROXY_JWT_ISSUER: &str = "nym-credential-proxy";

/// Why a free-tier token was issued: a new user trialing (granted the free
/// allowance) vs a returning user renewing (no free bandwidth, purchase-only).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FreeTierPurpose {
    NewUser,
    Renewal,
}

/// Custom claims of a free-tier capability JWT. A capability marker only; the
/// allowance is a network constant looked up at redemption.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FreeTierClaims {
    /// Why this token was issued; always set explicitly by the issuer.
    pub purpose: FreeTierPurpose,
}

impl FreeTierClaims {
    pub fn new(purpose: FreeTierPurpose) -> Self {
        FreeTierClaims { purpose }
    }
}

/// Mint a free-tier capability JWT signed by `keys`. Used by the credential
/// proxy to issue tokens and by tests to mint throwaway tokens.
pub fn generate_free_tier_jwt(
    validity: Duration,
    keys: &ed25519::KeyPair,
    issuer: Option<&str>,
    purpose: FreeTierPurpose,
) -> String {
    let claim = Claims::with_custom_claims(FreeTierClaims::new(purpose), validity.into());
    let mut claim = if let Some(issuer) = issuer {
        claim.with_issuer(issuer)
    } else {
        claim
    };
    claim.create_nonce();

    let md = KeyMetadata::default().with_public_key(keys.public_key().to_base58_string());
    let mut jwt_keys = keys.to_jwt_compatible_keys();
    // SAFETY: trait impl for EdDSA is infallible
    #[allow(clippy::unwrap_used)]
    jwt_keys.attach_metadata(md).unwrap();

    // SAFETY: our construction of the jwt is valid
    #[allow(clippy::unwrap_used)]
    jwt_keys.sign(claim).unwrap()
}

/// Verify a free-tier JWT offline against a configured attester public key.
/// No network, no attestation, no delegation: the token must be signed
/// directly by `attester_public_key`.
pub fn validate_free_tier_jwt(
    token: &str,
    attester_public_key: &ed25519::PublicKey,
    expected_issuer: Option<&str>,
) -> Result<FreeTierClaims, FreeTierCheckError> {
    let mut opts = VerificationOptions::default();
    if let Some(issuer) = expected_issuer {
        opts.allowed_issuers = Some(HashSet::from_iter(vec![issuer.to_string()]));
    }

    let claims = attester_public_key
        .to_jwt_compatible_key()
        .verify_token::<FreeTierClaims>(token, Some(opts))
        .map_err(|source| FreeTierCheckError::JwtVerificationFailure { source })?
        .custom;

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::ed25519;
    use nym_test_utils::helpers::deterministic_rng;

    fn hour() -> Duration {
        Duration::from_secs(60 * 60)
    }

    #[test]
    fn valid_token_accepted_with_configured_key() {
        let mut rng = deterministic_rng();
        let attester = ed25519::KeyPair::new(&mut rng);
        let jwt = generate_free_tier_jwt(
            hour(),
            &attester,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
            FreeTierPurpose::NewUser,
        );

        let claims = validate_free_tier_jwt(
            &jwt,
            attester.public_key(),
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
        )
        .expect("valid free-tier token should verify");
        assert_eq!(claims.purpose, FreeTierPurpose::NewUser);
    }

    #[test]
    fn token_signed_by_other_key_rejected() {
        let mut rng = deterministic_rng();
        let attester = ed25519::KeyPair::new(&mut rng);
        let impostor = ed25519::KeyPair::new(&mut rng);
        let jwt = generate_free_tier_jwt(
            hour(),
            &impostor,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
            FreeTierPurpose::NewUser,
        );

        assert!(
            validate_free_tier_jwt(
                &jwt,
                attester.public_key(),
                Some(CREDENTIAL_PROXY_JWT_ISSUER)
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_issuer_rejected() {
        let mut rng = deterministic_rng();
        let attester = ed25519::KeyPair::new(&mut rng);
        let jwt = generate_free_tier_jwt(
            hour(),
            &attester,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
            FreeTierPurpose::NewUser,
        );

        assert!(
            validate_free_tier_jwt(&jwt, attester.public_key(), Some("nym-someone-else")).is_err()
        );
    }

    #[test]
    fn any_issuer_accepted_when_not_required() {
        let mut rng = deterministic_rng();
        let attester = ed25519::KeyPair::new(&mut rng);
        let jwt = generate_free_tier_jwt(
            hour(),
            &attester,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
            FreeTierPurpose::NewUser,
        );

        assert!(validate_free_tier_jwt(&jwt, attester.public_key(), None).is_ok());
    }

    #[test]
    fn expired_token_rejected() {
        use jwt_simple::prelude::{Clock, Duration as JwtDuration};

        let mut rng = deterministic_rng();
        let attester = ed25519::KeyPair::new(&mut rng);

        // mint a token that already expired an hour ago
        let now = Clock::now_since_epoch();
        let mut claim = Claims::with_custom_claims(
            FreeTierClaims::new(FreeTierPurpose::NewUser),
            JwtDuration::from_secs(1),
        )
        .with_issuer(CREDENTIAL_PROXY_JWT_ISSUER);
        claim.issued_at = Some(now - JwtDuration::from_secs(2 * 60 * 60));
        claim.expires_at = Some(now - JwtDuration::from_secs(60 * 60));
        #[allow(clippy::unwrap_used)]
        let jwt = attester.to_jwt_compatible_keys().sign(claim).unwrap();

        assert!(
            validate_free_tier_jwt(
                &jwt,
                attester.public_key(),
                Some(CREDENTIAL_PROXY_JWT_ISSUER)
            )
            .is_err()
        );
    }

    #[test]
    fn purpose_roundtrips() {
        let mut rng = deterministic_rng();
        let attester = ed25519::KeyPair::new(&mut rng);

        let new_user = generate_free_tier_jwt(
            hour(),
            &attester,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
            FreeTierPurpose::NewUser,
        );
        let claims = validate_free_tier_jwt(&new_user, attester.public_key(), None).unwrap();
        assert_eq!(claims.purpose, FreeTierPurpose::NewUser);

        let renewal = generate_free_tier_jwt(
            hour(),
            &attester,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
            FreeTierPurpose::Renewal,
        );
        let claims = validate_free_tier_jwt(&renewal, attester.public_key(), None).unwrap();
        assert_eq!(claims.purpose, FreeTierPurpose::Renewal);
    }
}
