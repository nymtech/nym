use itertools::izip;

use crate::error::CompactEcashError;
use crate::scheme::aggregation::{aggregate_signature_shares, aggregate_verification_keys, aggregate_wallets};
use crate::scheme::keygen::{
    generate_keypair_user, PublicKeyUser, SecretKeyUser, ttp_keygen, VerificationKeyAuth,
};
use crate::scheme::PartialWallet;
use crate::scheme::PayInfo;
use crate::scheme::setup::Parameters;
use crate::scheme::withdrawal::{issue_verify, issue_wallet, withdrawal_request};
use crate::utils::SignatureShare;

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

    let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
        wallet_blinded_signatures.iter(),
        verification_keys_auth.iter()
    )
        .map(|(w, vk)| issue_verify(&params, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
        .collect();

    // Aggregate partial wallets
    let aggr_wallet = aggregate_wallets(&params, &verification_key, &user_keypair.secret_key(), &unblinded_wallet_shares, &req_info)?;

    // Let's try to spend some coins
    let payInfo = PayInfo {
        info: [6u8; 32],
    };

    let (payment, upd_wallet) = aggr_wallet.spend(&params, &verification_key, &user_keypair.secret_key(), &payInfo)?;

    assert!(payment.spend_verify(&params, &verification_key, &payInfo).unwrap());

    Ok(())
}
