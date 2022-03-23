use itertools::izip;

use crate::error::CompactEcashError;
use crate::scheme::{SignatureShare, Wallet};
use crate::scheme::aggregation::{aggregate_signature_shares, aggregate_verification_keys};
use crate::scheme::keygen::{
    generate_keypair_user, PublicKeyUser, SecretKeyUser, ttp_keygen, VerificationKeyAuth,
};
use crate::scheme::setup::Parameters;
use crate::scheme::withdrawal::{issue_verify, issue_wallet, withdrawal_request};

#[test]
fn main() -> Result<(), CompactEcashError> {
    let params = Parameters::new().unwrap();
    let user_keypair = generate_keypair_user(&params);

    let (req, req_info) = withdrawal_request(&params, &user_keypair.secret_key()).unwrap();
    let authorities_keypairs = ttp_keygen(&params, 2, 3).unwrap();

    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3]))?;

    let mut wallet_blinded_signatures = Vec::new();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue_wallet(
            &params,
            auth_keypair.secret_key(),
            user_keypair.public_key(),
            &req,
        );
        wallet_blinded_signatures.push(blind_signature.unwrap());
    }

    let unblinded_wallet_shares: Vec<Wallet> = izip!(
        wallet_blinded_signatures.iter(),
        verification_keys_auth.iter()
    )
        .map(|(w, vk)| issue_verify(&params, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
        .collect();

    // Aggregate partial wallets
    let signature_shares: Vec<SignatureShare> = unblinded_wallet_shares
        .iter()
        .enumerate()
        .map(|(idx, wallet)| SignatureShare::new(*wallet.signature(), (idx + 1) as u64))
        .collect();

    let attributes = vec![user_keypair.secret_key().sk, req_info.get_v(), req_info.get_t()];
    let aggregated_signature =
        aggregate_signature_shares(&params, &verification_key, &attributes, &signature_shares)?;

    Ok(())
}
