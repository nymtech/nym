use crate::scheme::verification::{prove_covid_credential, verify_covid_credential, ThetaCovid};
use crate::{
    aggregate_signature_shares, aggregate_verification_keys, blind_sign, elgamal_keygen,
    hash_to_scalar, prepare_blind_sign, setup, ttp_keygen, CoconutError, Signature, SignatureShare,
    VerificationKey,
};

#[test]
fn main() -> Result<(), CoconutError> {
    let params = setup(15)?;

    // validators keys
    let coconut_keypairs = ttp_keygen(&params, 2, 3)?;
    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&[1, 2, 3]))?;

    // user's ElGamal keypair
    let elgamal_keypair = elgamal_keygen(&params);

    // attributes to consider
    let patient_id = hash_to_scalar(String::from("NHS678777").as_bytes());
    let full_name = hash_to_scalar(String::from("JaneDoe").as_bytes());
    let vaccine_medication_product_id = hash_to_scalar(String::from("EU/1/20/1528").as_bytes());
    let country_of_vaccination = hash_to_scalar(String::from("UK").as_bytes());
    let issuer = hash_to_scalar(String::from("NHS").as_bytes());
    let dob = hash_to_scalar(String::from("2021-11-05").as_bytes());

    let public_attributes = vec![
        patient_id,
        full_name,
        vaccine_medication_product_id,
        country_of_vaccination,
        issuer,
        dob,
    ];
    let user_secret = params.random_scalar();
    let private_attributes = vec![user_secret];

    // ISSUANCE PROTOCOL
    let blind_sign_request = prepare_blind_sign(
        &params,
        &elgamal_keypair,
        &private_attributes,
        &public_attributes,
    )?;

    // generate blinded signatures
    let mut blinded_signatures = Vec::new();

    let is_vaccinated = hash_to_scalar(String::from("TRUE").as_bytes());
    let is_over_18 = hash_to_scalar(String::from("TRUE").as_bytes());
    let is_over_21 = hash_to_scalar(String::from("TRUE").as_bytes());

    // These are the attributes on which the validator issues a signature
    let public_attributes = [
        patient_id,
        full_name,
        vaccine_medication_product_id,
        country_of_vaccination,
        issuer,
        dob,
        is_vaccinated,
        is_over_18,
        is_over_21,
    ];

    for keypair in coconut_keypairs {
        let blinded_signature = blind_sign(
            &params,
            &keypair.secret_key(),
            &elgamal_keypair.public_key(),
            &blind_sign_request,
            &public_attributes,
        )?;
        blinded_signatures.push(blinded_signature)
    }

    let unblinded_signatures: Vec<Signature> = blinded_signatures
        .into_iter()
        .zip(verification_keys.iter())
        .map(|(signature, verification_key)| {
            signature
                .unblind(
                    &params,
                    &elgamal_keypair.private_key(),
                    &verification_key,
                    &private_attributes,
                    &public_attributes,
                    &blind_sign_request.get_commitment_hash(),
                )
                .unwrap()
        })
        .collect();

    let signature_shares: Vec<SignatureShare> = unblinded_signatures
        .iter()
        .enumerate()
        .map(|(idx, signature)| SignatureShare::new(*signature, (idx + 1) as u64))
        .collect();

    let mut attributes = Vec::with_capacity(1 + 9);
    attributes.extend_from_slice(&private_attributes);
    attributes.extend_from_slice(&public_attributes);

    // Randomize credentials and generate any cryptographic material to verify them
    let signature =
        aggregate_signature_shares(&params, &verification_key, &attributes, &signature_shares)?;

    // SHOW PROTOCOL
    let verifier_id = [11u8; 32];
    let timestamp = [12u8; 32];

    let show_private_attributes = vec![
        user_secret,
        patient_id,
        full_name,
        vaccine_medication_product_id,
        country_of_vaccination,
        issuer,
        dob,
    ];

    // Prove covid credential
    let theta_covid = prove_covid_credential(
        &params,
        &verification_key,
        &signature,
        &show_private_attributes,
        &verifier_id,
        &timestamp,
    )?;

    let theta_covid_bytes = theta_covid.to_bytes();
    println!("Length of theta in bytes: {:?}", theta_covid_bytes.len());

    let theta_covid_from_bytes = ThetaCovid::from_bytes(&*theta_covid_bytes).unwrap();

    // Verify covid credentials
    let disclosed_attributes = vec![is_vaccinated, is_over_18, is_over_21];
    assert!(verify_covid_credential(
        &params,
        &verification_key,
        &theta_covid_from_bytes,
        disclosed_attributes.as_ref(),
        &verifier_id,
        &timestamp,
    ));

    Ok(())
}
