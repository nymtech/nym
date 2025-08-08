use async_trait::async_trait;
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::{ClientTicket, VerificationKeyAuth};
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_validator_client::nym_api::EpochId;
use tokio::sync::RwLockReadGuard;

use crate::ecash::error::EcashTicketError;

#[async_trait]
pub trait EcashManager {
    async fn verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, EcashTicketError>;
    fn storage(&self) -> Box<dyn BandwidthGatewayStorage + Send + Sync>;
    async fn check_payment(
        &self,
        credential: &CredentialSpendingData,
        aggregated_verification_key: &VerificationKeyAuth,
    ) -> Result<(), EcashTicketError>;
    fn async_verify(&self, ticket: ClientTicket);
}
