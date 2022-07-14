use crate::error::{CompactEcashError, Result};
use crate::PayInfo;
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::Payment;

#[derive(Debug, Eq, PartialEq)]
pub enum IdentifyResult {
    NotADuplicatePayment,
    DuplicatePayInfo(PayInfo),
    DoubleSpendingPublicKeys(PublicKeyUser),
}

pub fn identify(public_keys_u: &[PublicKeyUser], pay1: Payment, pay2: Payment, pay_info1: PayInfo, pay_info2: PayInfo) -> Result<IdentifyResult> {
    let mut duplicate_serial_numbers: Vec<(u64, u64)> = Default::default();
    for k in 0..pay1.vv {
        for j in 0..pay2.vv {
            if pay1.ss[k as usize] == pay2.ss[j as usize] {
                duplicate_serial_numbers.push((k, j));
            }
        }
    }
    if duplicate_serial_numbers.is_empty() {
        return Ok(IdentifyResult::NotADuplicatePayment);
    } else {
        if pay_info1 == pay_info2 {
            return Ok(IdentifyResult::DuplicatePayInfo(pay_info1));
        } else {
            for elem in duplicate_serial_numbers.iter() {
                let k = elem.0 as usize;
                let j = elem.1 as usize;
                let pk = (pay2.tt[j] * pay1.rr[k] - pay1.tt[k] * pay2.rr[j]) * ((pay1.rr[k] - pay2.rr[j]).invert().unwrap());
                let pk_user = PublicKeyUser { pk: pk.clone() };
                if public_keys_u.contains(&pk_user) {
                    return Ok(IdentifyResult::DoubleSpendingPublicKeys(pk_user));
                } else {
                    return Err(CompactEcashError::Identify(
                        "A duplicate serial number was detected, the pay_info1 and pay_info2 are different, but we failed to identify the double-spending public key".to_string(),
                    ));
                }
            }
        }
        return Err(CompactEcashError::Identify(
            "A duplicate serial number was detected, the pay_info1 and pay_info2 are different, but we failed to identify the double-spending public key".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
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
            .spend_verify(&params, &verification_key, &pay_info1, spend_vv)
            .unwrap());

        let payment2 = payment1.clone();
        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info1, spend_vv)
            .unwrap());

        let pay_info2 = pay_info1.clone();
        let identify_result = identify(&[user_keypair.public_key()], payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
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
            .spend_verify(&params, &verification_key, &pay_info1, spend_vv)
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
            .spend_verify(&params, &verification_key, &pay_info2, spend_vv)
            .unwrap());

        let identify_result = identify(&[user_keypair.public_key()], payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::NotADuplicatePayment);
    }

    #[test]
    fn two_payments_with_one_repeating_serial_number_but_different_pay_info() {
        let L = 32;
        let params = setup(L);
        let grp = params.grp();
        let user_keypair = generate_keypair_user(&grp);

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

        let (payment1, upd_wallet) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info1,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1, spend_vv)
            .unwrap());

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = aggr_wallet.l.get();
        aggr_wallet.l.set(current_l - 1);

        let pay_info2 = PayInfo { info: [7u8; 32] };
        let spend_vv = 1;

        let (payment2, _) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info2,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info2, spend_vv)
            .unwrap());

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys: Vec<PublicKeyUser> = Default::default();
        for i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(&grp);
            public_keys.push(pk_user);
        }
        public_keys.push(user_keypair.public_key());


        let identify_result = identify(&public_keys, payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key()));
    }

    #[test]
    fn two_payments_with_multiple_repeating_serial_numbers_but_different_pay_info() {
        let L = 32;
        let params = setup(L);
        let grp = params.grp();
        let user_keypair = generate_keypair_user(&grp);

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

        let (payment1, upd_wallet) = aggr_wallet.spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info1,
            false,
            spend_vv,
        ).unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1, spend_vv)
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

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys: Vec<PublicKeyUser> = Default::default();
        for i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(&grp);
            public_keys.push(pk_user);
        }
        public_keys.push(user_keypair.public_key());

        let identify_result = identify(&public_keys, payment1, payment2, pay_info1.clone(), pay_info2.clone()).unwrap();
        assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key()));
    }
}