// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::*;
use itertools::izip;
use std::fmt::Debug;

// unwraps are fine in the test code
#[allow(clippy::unwrap_used)]
pub fn theta_from_keys_and_attributes(
    params: &Parameters,
    coconut_keypairs: &Vec<KeyPair>,
    indices: &[scheme::SignerIndex],
    public_attributes: &[&PublicAttribute],
) -> Result<Theta, CoconutError> {
    let serial_number = params.random_scalar();
    let binding_number = params.random_scalar();
    let private_attributes = vec![&serial_number, &binding_number];

    // generate commitment
    let (commitments_openings, blind_sign_request) =
        prepare_blind_sign(params, &private_attributes, public_attributes)?;

    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key().clone())
        .collect();

    // aggregate verification keys
    let verification_key = aggregate_verification_keys(&verification_keys, Some(indices))?;

    // generate blinded signatures
    let mut blinded_signatures = Vec::new();

    for keypair in coconut_keypairs {
        let blinded_signature = blind_sign(
            params,
            keypair.secret_key(),
            &blind_sign_request,
            public_attributes,
        )?;
        blinded_signatures.push(blinded_signature)
    }

    // Unblind
    let unblinded_signatures: Vec<(scheme::SignerIndex, Signature)> = izip!(
        indices.iter(),
        blinded_signatures.iter(),
        verification_keys.iter()
    )
    .map(|(idx, s, vk)| {
        (
            *idx,
            s.unblind_and_verify(
                params,
                vk,
                &private_attributes,
                public_attributes,
                &blind_sign_request.get_commitment_hash(),
                &commitments_openings,
            )
            .unwrap(),
        )
    })
    .collect();

    // Aggregate signatures
    let signature_shares: Vec<SignatureShare> = unblinded_signatures
        .iter()
        .map(|(idx, signature)| SignatureShare::new(*signature, *idx))
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
        &serial_number,
        &binding_number,
    )?;

    Ok(theta)
}

// unwraps are fine in the test code
#[allow(clippy::unwrap_used)]
pub fn transpose_matrix<T: Debug>(matrix: Vec<Vec<T>>) -> Vec<Vec<T>> {
    if matrix.is_empty() {
        return vec![];
    }
    let len = matrix[0].len();
    let mut iters: Vec<_> = matrix.into_iter().map(|d| d.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|it| it.next().unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

#[macro_export]
macro_rules! random_scalars_refs {
    ( $x: ident, $params: expr, $n: expr ) => {
        let _vec = $params.n_random_scalars($n);
        #[allow(clippy::map_identity)]
        let $x = _vec.iter().collect::<Vec<_>>();
    };
}

pub use random_scalars_refs;

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{KeyPair, Parameters, SecretKey};
    use bls12_381::Scalar;
    use nym_dkg::{bte::decrypt_share, combine_shares, Dealing, NodeIndex};
    use rand_chacha::rand_core::SeedableRng;

    pub fn generate_dkg_secrets(node_indices: &[NodeIndex]) -> Vec<Scalar> {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        let params = nym_dkg::bte::setup();

        // the simplest possible case
        let threshold = 2;

        let mut receivers = std::collections::BTreeMap::new();
        let mut full_keys = Vec::new();
        for index in node_indices {
            let (dk, pk) = nym_dkg::bte::keygen(&params, &mut rng);
            receivers.insert(*index, *pk.public_key());
            full_keys.push((dk, pk))
        }
        let dealings = node_indices
            .iter()
            .map(|&dealer_index| {
                Dealing::create(&mut rng, &params, dealer_index, threshold, &receivers, None).0
            })
            .collect::<Vec<_>>();
        let mut derived_secrets = Vec::new();
        for (i, (ref mut dk, _)) in full_keys.iter_mut().enumerate() {
            let shares = dealings
                .iter()
                .map(|dealing| decrypt_share(dk, i, &dealing.ciphertexts, None).unwrap())
                .collect();

            let recovered_secret =
                combine_shares(shares, &receivers.keys().copied().collect::<Vec<_>>()).unwrap();

            derived_secrets.push(recovered_secret)
        }
        derived_secrets
    }
    pub fn generate_dkg_keys(num_attributes: u32, node_indices: &[NodeIndex]) -> Vec<KeyPair> {
        let params = Parameters::new(num_attributes).unwrap();
        let mut all_secrets = vec![];
        for _ in 0..num_attributes {
            let secrets = generate_dkg_secrets(node_indices);
            all_secrets.push(secrets);
        }
        let signers = transpose_matrix(all_secrets);
        signers
            .into_iter()
            .map(|mut secrets| {
                let x = secrets.pop().unwrap();
                let sk = SecretKey::create_from_raw(x, secrets);
                let vk = sk.verification_key(&params);
                KeyPair::from_keys(sk, vk)
            })
            .collect()
    }
}
