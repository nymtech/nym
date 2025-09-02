// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{UpgradeModeAttestation, UpgradeModeCheckError};
use jwt_simple::claims::Claims;
use jwt_simple::common::{KeyMetadata, VerificationOptions};
use jwt_simple::prelude::{EdDSAKeyPairLike, EdDSAPublicKeyLike};
use jwt_simple::token::Token;
use nym_crypto::asymmetric::ed25519;
use std::collections::HashSet;
use std::time::Duration;

// for now use static issuer such as "nym-credential-proxy"
pub fn generate_jwt_for_upgrade_mode_attestation(
    attestation: UpgradeModeAttestation,
    validity: Duration,
    keys: &ed25519::KeyPair,
    issuer: Option<&'static str>,
) -> String {
    let claim = Claims::with_custom_claims(attestation, validity.into());
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

pub fn validate_upgrade_mode_jwt(
    token: &str,
    expected_issuer: Option<&'static str>,
) -> Result<UpgradeModeAttestation, UpgradeModeCheckError> {
    // for now, we completely ignore the validity of the pubkey (I know, I know).
    // that will be changed later on
    // so as a bypass we have to extract the claimed issuer from the jwt to verify against it
    let metadata = Token::decode_metadata(token)
        .map_err(|source| UpgradeModeCheckError::TokenMetadataDecodeFailure { source })?;

    let pub_key = metadata
        .public_key()
        .ok_or(UpgradeModeCheckError::MissingTokenPublicKey)?;

    let ed25519_pub_key = ed25519::PublicKey::from_base58_string(pub_key)
        .map_err(|source| UpgradeModeCheckError::MalformedEd25519PublicKey { source })?;

    let mut opts = VerificationOptions::default();
    if let Some(issuer) = expected_issuer {
        opts.allowed_issuers = Some(HashSet::from_iter(vec![issuer.to_string()]));
    }

    let attestation = ed25519_pub_key
        .to_jwt_compatible_key()
        .verify_token::<UpgradeModeAttestation>(token, Some(opts))
        .map_err(|source| UpgradeModeCheckError::JwtVerificationFailure { source })?
        .custom;

    Ok(attestation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_new_attestation;
    use nym_crypto::asymmetric::ed25519;

    #[test]
    fn generate_and_validate_jwt() {
        let attestation_key = ed25519::PrivateKey::from_bytes(&[
            108, 49, 193, 21, 126, 161, 249, 85, 242, 207, 74, 195, 238, 6, 64, 149, 201, 140, 248,
            163, 122, 170, 79, 198, 87, 85, 36, 29, 243, 92, 64, 161,
        ])
        .unwrap();
        let jwt_key = ed25519::PrivateKey::from_bytes(&[
            152, 17, 144, 255, 213, 219, 246, 208, 109, 33, 100, 73, 1, 141, 32, 63, 141, 89, 167,
            2, 52, 215, 241, 219, 200, 18, 159, 241, 76, 111, 42, 32,
        ])
        .unwrap();
        let keys = ed25519::KeyPair::from(jwt_key);

        let attestation = generate_new_attestation(&attestation_key);
        let jwt_issuer = generate_jwt_for_upgrade_mode_attestation(
            attestation,
            Duration::from_secs(60 * 60),
            &keys,
            Some("nym-credential-proxy"),
        );
        // we expect 'nym-credential-proxy' issuer
        assert!(validate_upgrade_mode_jwt(&jwt_issuer, Some("nym-credential-proxy")).is_ok());

        // we don't care about issuer
        assert!(validate_upgrade_mode_jwt(&jwt_issuer, None).is_ok());

        // we expect another-issuer
        assert!(validate_upgrade_mode_jwt(&jwt_issuer, Some("another-issuer")).is_err());

        let jwt_no_issuer = generate_jwt_for_upgrade_mode_attestation(
            attestation,
            Duration::from_secs(60 * 60),
            &keys,
            None,
        );
        // we expect 'nym-credential-proxy' issuer
        assert!(validate_upgrade_mode_jwt(&jwt_no_issuer, Some("nym-credential-proxy")).is_err());

        // we don't care about issuer
        assert!(validate_upgrade_mode_jwt(&jwt_no_issuer, None).is_ok());
    }
}
