// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::*;
use itertools::izip;

pub fn theta_from_keys_and_attributes(
    params: &Parameters,
    coconut_keypairs: &Vec<KeyPair>,
    public_attributes: &Vec<PublicAttribute>,
) -> Result<Theta, CoconutError> {
    let serial_number = params.random_scalar();
    let binding_number = params.random_scalar();
    let private_attributes = vec![serial_number, binding_number];

    // generate commitment
    let (commitments_openings, blind_sign_request) =
        prepare_blind_sign(params, &private_attributes, public_attributes)?;

    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    // aggregate verification keys
    let indices: Vec<u64> = coconut_keypairs
        .iter()
        .enumerate()
        .map(|(idx, _)| (idx + 1) as u64)
        .collect();
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&indices))?;

    // generate blinded signatures
    let mut blinded_signatures = Vec::new();

    for keypair in coconut_keypairs {
        let blinded_signature = blind_sign(
            params,
            &keypair.secret_key(),
            &blind_sign_request,
            public_attributes,
        )?;
        blinded_signatures.push(blinded_signature)
    }

    // Unblind
    let unblinded_signatures: Vec<Signature> =
        izip!(blinded_signatures.iter(), verification_keys.iter())
            .map(|(s, vk)| {
                s.unblind(
                    params,
                    vk,
                    &private_attributes,
                    public_attributes,
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
    attributes.extend_from_slice(public_attributes);

    // Randomize credentials and generate any cryptographic material to verify them
    let signature =
        aggregate_signature_shares(params, &verification_key, &attributes, &signature_shares)?;

    // Generate cryptographic material to verify them
    let theta = prove_bandwidth_credential(
        params,
        &verification_key,
        &signature,
        serial_number,
        binding_number,
    )?;

    Ok(theta)
}
