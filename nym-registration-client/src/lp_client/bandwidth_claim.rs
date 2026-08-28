// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::error::{LpClientError, Result};
use nym_authenticator_requests::models::BandwidthClaim;
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_credentials_interface::{BandwidthCredential, TicketType};
use nym_crypto::asymmetric::ed25519;
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::warn;

/// Builds the claim a dVPN registration is finalised with.
///
/// Spends a ticket whenever one is available. Only once the stock is exhausted does it fall back
/// to an upgrade-mode JWT, which is precisely the situation that token exists for: while the
/// network is undergoing an upgrade a gateway stops metering bandwidth, so a client holding no
/// tickets can still register. Without a token, exhaustion stays the error it has always been.
pub(crate) async fn produce_bandwidth_claim(
    bandwidth_provider: &dyn BandwidthTicketProvider,
    gateway_identity: ed25519::PublicKey,
    spend_time_skew: Option<TimeDuration>,
    ticket_type: TicketType,
) -> Result<BandwidthClaim> {
    let ticket = bandwidth_provider
        .get_ecash_ticket(
            ticket_type,
            gateway_identity,
            DEFAULT_TICKETS_TO_SPEND,
            OffsetDateTime::now_utc() - spend_time_skew.unwrap_or_default(),
        )
        .await
        .map_err(|e| {
            LpClientError::SendRegistrationRequest(format!(
                "Failed to acquire bandwidth credential: {e}",
            ))
        })?;

    // note that only an exhausted stock reaches for the token; a failed lookup is a real failure
    // and stays one
    if let Some(ticket) = ticket {
        return ticket
            .data
            .try_into()
            .map_err(|err| LpClientError::Other(format!("malformed stored credential: {err}")));
    }

    let token = bandwidth_provider
        .get_upgrade_mode_token()
        .await
        .map_err(|e| {
            LpClientError::SendRegistrationRequest(format!(
                "Failed to look up the upgrade mode token: {e}",
            ))
        })?
        .ok_or(LpClientError::NoTicketsAvailable {
            ticketbook_type: ticket_type,
        })?;

    warn!("out of {ticket_type} tickets - registering with the stored upgrade mode token instead");

    Ok(BandwidthClaim {
        credential: BandwidthCredential::UpgradeModeJWT { token },
        kind: ticket_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use nym_bandwidth_controller::error::BandwidthControllerError;
    use nym_bandwidth_controller::mock::MockBandwidthController;
    use nym_bandwidth_controller::{PreparedCredential, PreparedCredentialMetadata};

    /// A provider that is out of tickets, holding the given token (if any).
    struct ExhaustedTickets {
        upgrade_mode_token: Option<String>,
    }

    #[async_trait]
    impl BandwidthTicketProvider for ExhaustedTickets {
        async fn get_ecash_ticket(
            &self,
            _ticket_type: TicketType,
            _gateway_id: ed25519::PublicKey,
            _tickets_to_spend: u32,
            _spend_time: OffsetDateTime,
        ) -> std::result::Result<Option<PreparedCredential>, BandwidthControllerError> {
            Ok(None)
        }

        async fn get_upgrade_mode_token(
            &self,
        ) -> std::result::Result<Option<String>, BandwidthControllerError> {
            Ok(self.upgrade_mode_token.clone())
        }

        async fn attempt_revert_spending(
            &self,
            _metadata: PreparedCredentialMetadata,
        ) -> std::result::Result<bool, BandwidthControllerError> {
            Ok(true)
        }

        async fn close(&self) {}
    }

    /// A provider holding both a ticket and a token, so that which arm is taken is observable.
    #[derive(Default)]
    struct TicketAndToken {
        tickets: MockBandwidthController,
    }

    #[async_trait]
    impl BandwidthTicketProvider for TicketAndToken {
        async fn get_ecash_ticket(
            &self,
            ticket_type: TicketType,
            gateway_id: ed25519::PublicKey,
            tickets_to_spend: u32,
            spend_time: OffsetDateTime,
        ) -> std::result::Result<Option<PreparedCredential>, BandwidthControllerError> {
            self.tickets
                .get_ecash_ticket(ticket_type, gateway_id, tickets_to_spend, spend_time)
                .await
        }

        async fn get_upgrade_mode_token(
            &self,
        ) -> std::result::Result<Option<String>, BandwidthControllerError> {
            Ok(Some("token".to_string()))
        }

        async fn attempt_revert_spending(
            &self,
            _metadata: PreparedCredentialMetadata,
        ) -> std::result::Result<bool, BandwidthControllerError> {
            Ok(true)
        }

        async fn close(&self) {}
    }

    fn gateway_identity() -> ed25519::PublicKey {
        let mut rng = rand::rngs::OsRng;
        *ed25519::KeyPair::new(&mut rng).public_key()
    }

    #[tokio::test]
    async fn a_held_ticket_is_spent_rather_than_the_token() {
        // the provider holds both, so this fails if the arms are ever reordered
        let claim = produce_bandwidth_claim(
            &TicketAndToken::default(),
            gateway_identity(),
            None,
            TicketType::V1WireguardEntry,
        )
        .await
        .unwrap();

        assert!(matches!(claim.credential, BandwidthCredential::ZkNym(_)));
        assert_eq!(TicketType::V1WireguardEntry, claim.kind);
    }

    #[tokio::test]
    async fn an_exhausted_stock_falls_back_to_the_token() {
        let provider = ExhaustedTickets {
            upgrade_mode_token: Some("token".to_string()),
        };

        let claim = produce_bandwidth_claim(
            &provider,
            gateway_identity(),
            None,
            TicketType::V1WireguardEntry,
        )
        .await
        .unwrap();

        assert!(matches!(
            claim.credential,
            BandwidthCredential::UpgradeModeJWT { token } if token == "token"
        ));
        // the claim is still tagged with the type that was asked for
        assert_eq!(TicketType::V1WireguardEntry, claim.kind);
    }

    #[tokio::test]
    async fn an_exhausted_stock_without_a_token_stays_the_error_it_was() {
        let provider = ExhaustedTickets {
            upgrade_mode_token: None,
        };

        let err = produce_bandwidth_claim(
            &provider,
            gateway_identity(),
            None,
            TicketType::V1WireguardEntry,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            err,
            LpClientError::NoTicketsAvailable { ticketbook_type }
                if ticketbook_type == TicketType::V1WireguardEntry
        ));
    }
}
