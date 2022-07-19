use std::collections::HashSet;

use bls12_381::G1Projective;
use group::Curve;

use crate::{PayInfo, VerificationKeyAuth};
use crate::error::{CompactEcashError, Result};
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::Payment;
use crate::scheme::setup::Parameters;

#[derive(Debug, Eq, PartialEq)]
pub enum IdentifyResult {
    NotADuplicatePayment,
    DuplicatePayInfo(PayInfo),
    DoubleSpendingPublicKeys(PublicKeyUser),
}


pub fn identify(params: &Parameters, public_keys_u: &[PublicKeyUser], verification_key: &VerificationKeyAuth, payment1: Payment, payment2: Payment, pay_info1: PayInfo, pay_info2: PayInfo) -> Result<IdentifyResult> {
    //  verify first the validity of both payments
    assert!(payment1.spend_verify(&params, &verification_key, &pay_info1).unwrap());
    assert!(payment2.spend_verify(&params, &verification_key, &pay_info2).unwrap());

    let mut k = 0;
    let mut j = 0;
    'outer: for (id1, pay1_ss) in payment1.ss.iter().enumerate() {
        'inner: for (id2, pay2_ss) in payment2.ss.iter().enumerate() {
            if pay1_ss == pay2_ss {
                k = id1.clone();
                j = id2.clone();
                break 'outer;
            }
        }
        return Ok(IdentifyResult::NotADuplicatePayment);
    }
    return if pay_info1 == pay_info2 {
        Ok(IdentifyResult::DuplicatePayInfo(pay_info1))
    } else {
        let pk = (payment2.tt[j] * payment1.rr[k] - payment1.tt[k] * payment2.rr[j]) * ((payment1.rr[k] - payment2.rr[j]).invert().unwrap());
        let pk_user = PublicKeyUser { pk: pk.clone() };
        if public_keys_u.contains(&pk_user) {
            Ok(IdentifyResult::DoubleSpendingPublicKeys(pk_user))
        } else {
            Err(CompactEcashError::Identify(
                "A duplicate serial number was detected, the pay_info1 and pay_info2 are different, but we failed to identify the double-spending public key".to_string(),
            ))
        }
    };
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use group::Curve;
    use itertools::izip;

    use crate::{aggregate_verification_keys, aggregate_wallets, generate_keypair_user, issue_verify, issue_wallet, PartialWallet, PayInfo, ttp_keygen, VerificationKeyAuth, withdrawal_request};
    use crate::scheme::identify::{identify, IdentifyResult};
    use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser};
    use crate::scheme::setup::setup;

    #[test]
    fn duplicate_payments_with_the_same_pay_info() {
        let L = 32;
        let params = setup(L);
        let grparams = params.grp();
        let user_keypair = generate_keypair_user(&grparams);

        let (req, req_info) = withdrawal_request(grparams, &user_keypair.secret_key()).unwrap();
        let authorities_keypairs = ttp_keygen(&grparams, 2, 3).unwrap();

        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

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

        // Aggregate partial wallets
        let aggr_wallet = aggregate_wallets(
            &grparams,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        ).unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo { info: [6u8; 32] };
        let spend_vv = 1;

        let (payment1, upd_wallet) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info1,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1)
            .unwrap());

        let payment2 = payment1.clone();
        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info1)
            .unwrap());

        let pay_info2 = pay_info1.clone();
        let identify_result = identify(&params, &[user_keypair.public_key()], &verification_key, payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::DuplicatePayInfo(pay_info1.clone()));
    }

    #[test]
    fn ok_if_two_different_payments() {
        let L = 32;
        let params = setup(L);
        let grparams = params.grp();
        let user_keypair = generate_keypair_user(&grparams);

        let (req, req_info) = withdrawal_request(grparams, &user_keypair.secret_key()).unwrap();
        let authorities_keypairs = ttp_keygen(&grparams, 2, 3).unwrap();

        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

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

        // Aggregate partial wallets
        let aggr_wallet = aggregate_wallets(
            &grparams,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        ).unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo { info: [6u8; 32] };
        let spend_vv = 1;

        let (payment1, upd_wallet) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info1,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1)
            .unwrap());


        let pay_info2 = PayInfo { info: [7u8; 32] };
        let (payment2, _) = upd_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info2,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info2)
            .unwrap());

        let identify_result = identify(&params, &[user_keypair.public_key()], &verification_key, payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::NotADuplicatePayment);
    }

    #[test]
    fn two_payments_with_one_repeating_serial_number_but_different_pay_info() {
        let L = 32;
        let params = setup(L);
        let grp = params.grp();
        let user_keypair = generate_keypair_user(&grp);

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys: Vec<PublicKeyUser> = Default::default();
        for i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(&grp);
            public_keys.push(pk_user.clone());
        }
        public_keys.push(user_keypair.public_key().clone());


        let (req, req_info) = withdrawal_request(grp, &user_keypair.secret_key()).unwrap();
        let authorities_keypairs = ttp_keygen(&grp, 2, 3).unwrap();

        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue_wallet(
                &grp,
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
            .map(|(w, vk)| issue_verify(&grp, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
            .collect();

        // Aggregate partial wallets
        let aggr_wallet = aggregate_wallets(
            &grp,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        ).unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo { info: [6u8; 32] };
        let spend_vv = 1;

        let (payment1, _) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info1,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1)
            .unwrap());

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = aggr_wallet.l.get();
        aggr_wallet.l.set(current_l - 1);

        let pay_info2 = PayInfo { info: [7u8; 32] };

        let (payment2, _) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info2,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info2)
            .unwrap());

        let identify_result = identify(&params, &public_keys, &verification_key, payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key()));
    }

    #[test]
    fn two_payments_with_multiple_repeating_serial_numbers_but_different_pay_info() {
        let L = 32;
        let params = setup(L);
        let grp = params.grp();
        let user_keypair = generate_keypair_user(&grp);

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys: Vec<PublicKeyUser> = Default::default();
        for i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(&grp);
            public_keys.push(pk_user.clone());
        }
        public_keys.push(user_keypair.public_key().clone());

        let (req, req_info) = withdrawal_request(grp, &user_keypair.secret_key()).unwrap();
        let authorities_keypairs = ttp_keygen(&grp, 2, 3).unwrap();

        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue_wallet(
                &grp,
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
            .map(|(w, vk)| issue_verify(&grp, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
            .collect();

        // Aggregate partial wallets
        let aggr_wallet = aggregate_wallets(
            &grp,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        ).unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo { info: [6u8; 32] };
        let spend_vv = 10;

        let (payment1, _) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info1,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1)
            .unwrap());

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = aggr_wallet.l.get();
        aggr_wallet.l.set(current_l - 10);

        let pay_info2 = PayInfo { info: [7u8; 32] };
        let (payment2, _) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info2,
            false,
            spend_vv,
        ).unwrap();


        let identify_result = identify(&params, &public_keys, &verification_key, payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key()));
    }
}