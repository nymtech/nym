// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_upgrade_mode_check::{
    CREDENTIAL_PROXY_JWT_ISSUER, UpgradeModeAttestation, generate_jwt_for_upgrade_mode_attestation,
};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::error;

#[derive(Debug, Clone)]
pub(crate) struct UpgradeModeState {
    pub(crate) inner: Arc<RwLock<Option<UpgradeModeStateInner>>>,
}

impl UpgradeModeState {
    pub(crate) async fn has_attestation(&self) -> bool {
        self.inner.read().await.is_some()
    }

    pub(crate) async fn update(
        &self,
        retrieved_attestation: Option<UpgradeModeAttestation>,
        expected_attester_public_key: ed25519::PublicKey,
        jwt_signing_keys: &ed25519::KeyPair,
        jwt_validity: Duration,
    ) {
        let mut guard = self.inner.write().await;
        let Some(attestation) = retrieved_attestation else {
            *guard = None;
            return;
        };

        if attestation.content.attester_public_key != expected_attester_public_key {
            error!(
                "the retrieved attestation has been signed with an unexpected key! expected pubkey: {} actual: {}",
                expected_attester_public_key, attestation.content.attester_public_key
            );
            return;
        }

        match guard.as_mut() {
            None => {
                // no existing state - it's the first time we're going into upgrade mode,
                // so generate the jwt
                *guard = Some(UpgradeModeStateInner::new_fresh(
                    attestation,
                    jwt_signing_keys,
                    jwt_validity,
                ));
            }
            Some(current_state) => {
                let mut should_refresh = false;
                // update the jwt if we have issued one and:
                // - either the attestation has changed
                // - or the existing jwt is close to expiry
                if current_state.attestation != attestation {
                    should_refresh = true;
                }

                if let Some(issued_jwt) = current_state.jwt.as_ref()
                    && issued_jwt.close_to_expiry()
                {
                    should_refresh = true;
                }

                if should_refresh {
                    current_state.attestation = attestation;
                    current_state.refresh_jwt(jwt_signing_keys, jwt_validity);
                }
            }
        }
    }

    pub(crate) async fn attestation_with_jwt(
        &self,
    ) -> Option<(UpgradeModeAttestation, Option<String>)> {
        let guard = self.inner.read().await;
        let inner = guard.as_ref()?;
        Some((
            inner.attestation.clone(),
            inner.jwt.as_ref().map(|jwt| jwt.token.clone()),
        ))
    }
}

#[derive(Debug)]
pub(crate) struct UpgradeModeStateInner {
    pub(crate) attestation: UpgradeModeAttestation,
    pub(crate) jwt: Option<Jwt>,
}

impl UpgradeModeStateInner {
    fn try_generate_jwt(
        attestation: &UpgradeModeAttestation,
        jwt_signing_keys: &ed25519::KeyPair,
        jwt_validity: Duration,
    ) -> Option<Jwt> {
        if attestation.authorised_to_issue_jwt(jwt_signing_keys.public_key()) {
            Some(Jwt::generate(attestation, jwt_signing_keys, jwt_validity))
        } else {
            None
        }
    }

    fn new_fresh(
        attestation: UpgradeModeAttestation,
        jwt_signing_keys: &ed25519::KeyPair,
        jwt_validity: Duration,
    ) -> Self {
        let jwt = Self::try_generate_jwt(&attestation, jwt_signing_keys, jwt_validity);
        UpgradeModeStateInner { attestation, jwt }
    }

    fn refresh_jwt(&mut self, keys: &ed25519::KeyPair, validity: Duration) {
        self.jwt = Self::try_generate_jwt(&self.attestation, keys, validity);
    }
}

#[derive(Debug)]
pub(crate) struct Jwt {
    pub(crate) issued_at: OffsetDateTime,
    pub(crate) issued_for: Duration,
    pub(crate) token: String,
}

impl Jwt {
    fn generate(
        upgrade_mode_attestation: &UpgradeModeAttestation,
        keys: &ed25519::KeyPair,
        validity: Duration,
    ) -> Self {
        Jwt {
            issued_at: OffsetDateTime::now_utc(),
            issued_for: validity,
            token: generate_jwt_for_upgrade_mode_attestation(
                upgrade_mode_attestation.clone(),
                validity,
                keys,
                Some(CREDENTIAL_PROXY_JWT_ISSUER),
            ),
        }
    }

    fn close_to_expiry(&self) -> bool {
        // less than 20% of validity left
        let now = OffsetDateTime::now_utc();
        let validity_threshold = Duration::from_secs_f32(self.issued_for.as_secs_f32() * 0.8);
        now - self.issued_at >= validity_threshold
    }
}
