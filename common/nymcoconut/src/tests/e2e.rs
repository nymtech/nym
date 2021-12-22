use crate::{
    aggregate_signature_shares, aggregate_verification_keys, blind_sign, prepare_blind_sign,
    prove_bandwidth_credential, setup, ttp_keygen, verify_credential, CoconutError, Signature,
    SignatureShare, VerificationKey,
};

use bls12_381::G1Projective;
use itertools::izip;

#[test]
fn main() -> Result<(), CoconutError> {
    let params = setup(5)?;

    let public_attributes = params.n_random_scalars(2);
    let serial_number = params.random_scalar();
    let binding_number = params.random_scalar();
    let private_attributes = vec![serial_number, binding_number];
    let commitments_openings = params.n_random_scalars(2);

    // generate commitment and encryption
    let blind_sign_request = prepare_blind_sign(
        &params,
        &private_attributes,
        &commitments_openings,
        &public_attributes,
    )?;

    // generate_keys
    let coconut_keypairs = ttp_keygen(&params, 2, 3)?;

    let betas_g1: Vec<Vec<G1Projective>> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.secret_key().betas_g1(&params))
        .collect();
    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    // aggregate verification keys
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&[1, 2, 3]))?;

    // generate blinded signatures
    let mut blinded_signatures = Vec::new();

    for keypair in coconut_keypairs {
        let blinded_signature = blind_sign(
            &params,
            &keypair.secret_key(),
            &blind_sign_request,
            &public_attributes,
        )?;
        blinded_signatures.push(blinded_signature)
    }

    // Unblind
    let unblinded_signatures: Vec<Signature> = izip!(
        blinded_signatures.iter(),
        betas_g1.iter(),
        verification_keys.iter()
    )
    .map(|(s, b, vk)| {
        s.unblind(
            &params,
            &b,
            &vk,
            &private_attributes,
            &public_attributes,
            &blind_sign_request.get_commitment_hash(),
            &commitments_openings,
        )
        .unwrap()
    })
    .collect();

    // Aggregate signatures
    let signature_shares: Vec<SignatureShare> = unblinded_signatures
        .iter()
        .enumerate()
        .map(|(idx, signature)| SignatureShare::new(*signature, (idx + 1) as u64))
        .collect();

    let mut attributes = Vec::with_capacity(private_attributes.len() + public_attributes.len());
    attributes.extend_from_slice(&private_attributes);
    attributes.extend_from_slice(&public_attributes);

    // Randomize credentials and generate any cryptographic material to verify them
    let signature =
        aggregate_signature_shares(&params, &verification_key, &attributes, &signature_shares)?;

    // Generate cryptographic material to verify them
    let theta = prove_bandwidth_credential(
        &params,
        &verification_key,
        &signature,
        serial_number,
        binding_number,
    )?;

    // Verify credentials
    assert!(verify_credential(
        &params,
        &verification_key,
        &theta,
        &public_attributes,
    ));

    Ok(())
}
