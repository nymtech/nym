use crate::{
    aggregate_verification_keys, setup, tests::helpers::theta_from_keys_and_attributes, ttp_keygen,
    verify_credential, CoconutError, VerificationKey,
};

#[test]
fn main() -> Result<(), CoconutError> {
    let params = setup(5)?;

    let public_attributes = params.n_random_scalars(2);

    // generate_keys
    let coconut_keypairs = ttp_keygen(&params, 2, 3)?;

    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    // aggregate verification keys
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&[1, 2, 3]))?;

    // Generate cryptographic material to verify them
    let theta = theta_from_keys_and_attributes(&params, &coconut_keypairs, &public_attributes)?;

    // Verify credentials
    assert!(verify_credential(
        &params,
        &verification_key,
        &theta,
        &public_attributes,
    ));

    Ok(())
}
