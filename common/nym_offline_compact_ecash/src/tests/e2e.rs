use itertools::izip;

use crate::error::CompactEcashError;
use crate::scheme::aggregation::{
    aggregate_verification_keys, aggregate_wallets,
};
use crate::scheme::keygen::{
    generate_keypair_user, ttp_keygen, VerificationKeyAuth,
};
use crate::scheme::{Wallet, PartialWallet, Payment};
use crate::scheme::PayInfo;
use crate::scheme::setup::setup;
use crate::scheme::withdrawal::{issue_verify, issue_wallet, withdrawal_request, WithdrawalRequest};

#[test]
fn main() -> Result<(), CompactEcashError> {
    let L = 32;
    let params = setup(L);
    let grparams = params.grp();
    let user_keypair = generate_keypair_user(&grparams);

    let (req, req_info) = withdrawal_request(grparams, &user_keypair.secret_key()).unwrap();
    let req_bytes = req.to_bytes();
    let req2 = WithdrawalRequest::try_from(req_bytes.as_slice()).unwrap();
    assert_eq!(req, req2);

    let authorities_keypairs = ttp_keygen(&grparams, 2, 3).unwrap();

    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3]))?;

    let mut wallet_blinded_signatures = Vec::new();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue_wallet(
            &grparams,
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
        .map(|(w, vk)| issue_verify(&grparams, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
        .collect();

    let partial_wallet = unblinded_wallet_shares.get(0).unwrap().clone();
    let partial_wallet_bytes = partial_wallet.to_bytes();
    let partial_wallet2 = PartialWallet::try_from(&partial_wallet_bytes[..]).unwrap();
    assert_eq!(partial_wallet, partial_wallet2);

    // Aggregate partial wallets
    let aggr_wallet = aggregate_wallets(
        &grparams,
        &verification_key,
        &user_keypair.secret_key(),
        &unblinded_wallet_shares,
        &req_info,
    )?;

    let wallet_bytes = aggr_wallet.to_bytes();
    let wallet = Wallet::try_from(&wallet_bytes[..]).unwrap();
    assert_eq!(aggr_wallet, wallet);

    // Let's try to spend some coins
    let provider_keypair = generate_keypair_user(&grparams);
    let payinfo = PayInfo::generate_payinfo(provider_keypair.public_key());
    let spend_vv = 1;

    let (payment, _) = aggr_wallet.spend(
        &params,
        &verification_key,
        &user_keypair.secret_key(),
        &payinfo,
        false,
        spend_vv,
    )?;

    assert!(payment
        .spend_verify(&params, &verification_key, &payinfo)
        .unwrap());

    let payment_bytes = payment.to_bytes();
    let payment2 = Payment::try_from(&payment_bytes[..]).unwrap();
    assert_eq!(payment, payment2);

    Ok(())
}
