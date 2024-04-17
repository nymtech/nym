use nym_bandwidth_controller::PreparedCredential;
use nym_credentials::{
    coconut::bandwidth::bandwidth_credential_params, obtain_aggregate_verification_key,
    IssuedBandwidthCredential,
};
use nym_validator_client::coconut::CoconutApiClient;

use crate::Result;

pub(super) async fn verify_credential(
    valid_credential: &IssuedBandwidthCredential,
    credential_id: i64,
    coconut_api_clients: Vec<CoconutApiClient>,
) -> Result<()> {
    let verification_key = obtain_aggregate_verification_key(&coconut_api_clients).unwrap();
    let spend_request = valid_credential
        .prepare_for_spending(&verification_key)
        .unwrap();
    let prepared_credential = PreparedCredential {
        data: spend_request,
        epoch_id: valid_credential.epoch_id(),
        credential_id,
    };

    if !prepared_credential.data.validate_type_attribute() {
        panic!("missing bandwidth type attribute");
    }

    let params = bandwidth_credential_params();
    if prepared_credential.data.verify(params, &verification_key) {
        log::info!("Successfully validated credential");
        Ok(())
    } else {
        panic!("failed to validate credential");
    }
}
