use rand::thread_rng;

use crate::error::DivisibleEcashError;
use crate::scheme::{PayInfo, Payment};
use crate::scheme::aggregation::{aggregate_signatures, aggregate_verification_keys, aggregate_wallets};
use crate::scheme::identification::identify;
use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser, ttp_keygen_authorities, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::scheme::withdrawal::{issue, issue_verify, withdrawal_request};

#[test]
// Test wa full end to end flow of withdrawal request, issuance,
// and spending.
fn main() -> Result<(), DivisibleEcashError> {
    // SETUP PHASE
    let grp = GroupParameters::new().unwrap();
    let params = Parameters::new(grp.clone());

    // KEY GENERATION FOR THE AUTHORITIES
    let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let verification_key =
        aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

    // KEY GENERATION FOR THE USER
    let sk = grp.random_scalar();
    let sk_user = SecretKeyUser { sk };
    let pk_user = SecretKeyUser::public_key(&sk_user, &grp);

    // WITHDRAWAL REQUEST
    let (withdrawal_req, req_info) = withdrawal_request(&params, &sk_user)?;

    // ISSUE PARTIAL WALLETS
    let mut partial_wallets = Vec::new();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue(
            &params,
            &withdrawal_req,
            pk_user.clone(),
            &auth_keypair.secret_key(),
        )?;
        let partial_wallet = issue_verify(&grp, &auth_keypair.verification_key(), &sk_user, &blind_signature, &req_info)?;
        partial_wallets.push(partial_wallet);
    }

    // AGGREGATE WALLET
    let mut wallet = aggregate_wallets(&grp, &verification_key, &sk_user, &partial_wallets)?;

    let pay_info = PayInfo { info: [67u8; 32] };
    let (payment, wallet) = wallet.spend(&params, &verification_key, &sk_user, &pay_info, 10, false)?;

    // SPEND VERIFICATION 
    assert!(payment.spend_verify(&params, &verification_key, &pay_info).unwrap());
    let payment_bytes = payment.to_bytes();
    let payment2 = Payment::try_from(&payment_bytes[..]).unwrap();
    assert_eq!(payment, payment2);

    Ok(())
}
