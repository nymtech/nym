use std::collections::{HashMap, HashSet};
use std::ops::Neg;

use bls12_381::{Gt, pairing, Scalar};
use group::Curve;

use crate::error::{DivisibleEcashError, Result};
use crate::scheme::{PayInfo, Payment};
use crate::scheme::identification::IdentifyResult::DoubleSpendingPublicKeys;
use crate::scheme::keygen::{PublicKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;

#[derive(Debug, Eq, PartialEq)]
pub enum IdentifyResult {
    NotADuplicatePayment,
    DuplicatePayInfo(PayInfo),
    DoubleSpendingPublicKeys(PublicKeyUser),
    Whatever,
}

// how do we get the list of all pkU ?
pub fn identify(
    params: &Parameters,
    verification_key: &VerificationKeyAuth,
    public_keys_u: &HashSet<PublicKeyUser>,
    payment1: Payment,
    payment2: Payment,
    pay_info1: PayInfo,
    pay_info2: PayInfo) -> Result<IdentifyResult> {
    let params_a = params.get_params_a();

    // compute the serial numbers for k1 in [0, V1-1]
    let mut serial_numbers = HashMap::new();

    for k in 0..payment1.vv {
        let sn = pairing(&payment1.phi.1.to_affine(), &params_a.get_ith_delta(k as usize).to_affine())
            + pairing(&payment1.phi.0.to_affine(), &params_a.get_etas_ith_jth_elem(payment1.vv as usize, k as usize).to_affine());
        serial_numbers.insert(sn, k);
    }

    // compute the serial numbers fo k2 in [0, V2-1]
    let mut k1 = 0;
    let mut k2 = 0;
    let mut duplicate_serial_numbers: Vec<(Gt, u64, u64)> = Default::default();
    for j in 0..payment2.vv {
        let sn = pairing(&payment2.phi.1.to_affine(), &params_a.get_ith_delta(j as usize).to_affine())
            + pairing(&payment2.phi.0.to_affine(), &params_a.get_etas_ith_jth_elem(payment2.vv as usize, j as usize).to_affine());
        if !serial_numbers.contains_key(&sn) {
            serial_numbers.insert(sn, j);
        } else {
            k1 = *serial_numbers.get(&sn).unwrap() as u64;
            k2 = j.clone();
            break;
        }
        return Ok(IdentifyResult::NotADuplicatePayment);
    }

    if pay_info1 == pay_info2 {
        Ok(IdentifyResult::DuplicatePayInfo(pay_info1))
    } else {
        let delta_k1 = params_a.get_ith_delta(k1 as usize);
        let delta_k2 = params_a.get_ith_delta(k2 as usize);
        let tt1 = pairing(&payment1.varphi.1.to_affine(), &delta_k1.to_affine())
            + pairing(&payment1.varphi.0.to_affine(), &params_a.get_etas_ith_jth_elem(payment1.vv as usize, k1 as usize).to_affine());
        let tt2 = pairing(&payment2.varphi.1.to_affine(), &delta_k2.to_affine())
            + pairing(&payment2.varphi.0.to_affine(), &params_a.get_etas_ith_jth_elem(payment2.vv as usize, k2 as usize).to_affine());


        for pk_u in public_keys_u.iter() {
            let pg_pku_deltas = pairing(&pk_u.pk.to_affine(), &(delta_k1 * payment1.rr + delta_k2 * payment2.rr.neg()).to_affine());
            if tt1 - tt2 == pg_pku_deltas {
                return Ok(IdentifyResult::DoubleSpendingPublicKeys(pk_u.clone()));
            }
        }
        return Err(DivisibleEcashError::Identify(
            "A duplicate serial number was detected, the payinfo1 and payinfo2 are different, but we failed to identify the double-spending public key".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bls12_381::pairing;
    use group::Curve;
    use rand::thread_rng;

    use crate::scheme::{PayInfo, Payment};
    use crate::scheme::aggregation::{aggregate_verification_keys, aggregate_wallets};
    use crate::scheme::identification::{identify, IdentifyResult};
    use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser, ttp_keygen_authorities, VerificationKeyAuth};
    use crate::scheme::setup::{GroupParameters, Parameters};
    use crate::scheme::withdrawal::{issue, issue_verify, withdrawal_request};
    use crate::utils::hash_g1;

    #[test]
    fn duplicate_payments_with_the_same_pay_info() {
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        // KEY GENERATION FOR THE AUTHORITIES
        let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // KEY GENERATION FOR THE USER1
        let sk1 = grp.random_scalar();
        let sk_user1 = SecretKeyUser { sk: sk1 };
        let pk_user1 = SecretKeyUser::public_key(&sk_user1, &grp);

        // KEY GENERATION FOR THE USER2
        let sk2 = grp.random_scalar();
        let sk_user2 = SecretKeyUser { sk: sk2 };
        let pk_user2 = SecretKeyUser::public_key(&sk_user2, &grp);


        // WITHDRAWAL REQUEST FOR USER1
        let (withdrawal_req1, req_info1) = withdrawal_request(&params, &sk_user1).unwrap();

        // ISSUE PARTIAL WALLETS for USER1
        let mut partial_wallets1 = Vec::new();
        for auth_keypair in authorities_keypairs.clone() {
            let blind_signature = issue(
                &params,
                &withdrawal_req1,
                pk_user1.clone(),
                &auth_keypair.secret_key(),
            ).unwrap();
            let partial_wallet1 = issue_verify(&grp, &auth_keypair.verification_key(), &sk_user1, &blind_signature, &req_info1).unwrap();
            partial_wallets1.push(partial_wallet1);
        }

        // AGGREGATE WALLET FOR USER1
        let mut wallet1 = aggregate_wallets(&grp, &verification_key, &sk_user1, &partial_wallets1).unwrap();

        let pay_info1 = PayInfo { info: [67u8; 32] };
        let (payment1, wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info1, 10, false).unwrap();

        // SPEND VERIFICATION for USER1
        assert!(payment1.spend_verify(&params, &verification_key, &pay_info1).unwrap());

        let payment2 = payment1.clone();
        // SPEND VERIFICATION for the duplicate payment
        assert!(payment1.spend_verify(&params, &verification_key, &pay_info1).unwrap());

        let pay_info2 = pay_info1.clone();

        let public_keys = HashSet::from([pk_user1, pk_user2]);
        let identify_result = identify(&params, &verification_key, &public_keys, payment1, payment2, pay_info1, pay_info2).unwrap();
        assert_eq!(identify_result, IdentifyResult::DuplicatePayInfo(pay_info1));
    }

    #[test]
    fn two_payments_with_one_repeating_serial_number_but_different_pay_info() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        // KEY GENERATION FOR THE AUTHORITIES
        let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // KEY GENERATION FOR THE USER1
        let sk1 = grp.random_scalar();
        let sk_user1 = SecretKeyUser { sk: sk1 };
        let pk_user1 = SecretKeyUser::public_key(&sk_user1, &grp);


        //  GENERATE KEYS FOR OTHER USERS
        let mut pk_all_users = HashSet::new();
        for i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(&grp);
            pk_all_users.insert(pk_user);
        }
        pk_all_users.insert(pk_user1.clone());

        // WITHDRAWAL REQUEST FOR USER1
        let (withdrawal_req1, req_info1) = withdrawal_request(&params, &sk_user1).unwrap();

        // ISSUE PARTIAL WALLETS for USER1
        let mut partial_wallets1 = Vec::new();
        for auth_keypair in authorities_keypairs.clone() {
            let blind_signature = issue(
                &params,
                &withdrawal_req1,
                pk_user1.clone(),
                &auth_keypair.secret_key(),
            ).unwrap();
            let partial_wallet1 = issue_verify(&grp, &auth_keypair.verification_key(), &sk_user1, &blind_signature, &req_info1).unwrap();
            partial_wallets1.push(partial_wallet1);
        }

        // AGGREGATE WALLET FOR USER1
        let mut wallet1 = aggregate_wallets(&grp, &verification_key, &sk_user1, &partial_wallets1).unwrap();

        let pay_info1 = PayInfo { info: [67u8; 32] };
        let (payment1, new_wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info1, 10, false).unwrap();

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = wallet1.l.get();
        wallet1.l.set(current_l - 1);

        let pay_info2 = PayInfo { info: [52u8; 32] };
        let (payment2, wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info2, 10, false).unwrap();


        let identify_result = identify(&params, &verification_key, &pk_all_users, payment1, payment2, pay_info1, pay_info2).unwrap();

        assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(pk_user1));
    }

    #[test]
    fn two_payments_with_multiple_repeating_serial_number_but_different_pay_info() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        // KEY GENERATION FOR THE AUTHORITIES
        let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // KEY GENERATION FOR THE USER1
        let sk1 = grp.random_scalar();
        let sk_user1 = SecretKeyUser { sk: sk1 };
        let pk_user1 = SecretKeyUser::public_key(&sk_user1, &grp);

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys = HashSet::new();
        for i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(&grp);
            public_keys.insert(pk_user);
        }
        public_keys.insert(pk_user1.clone());

        // WITHDRAWAL REQUEST FOR USER1
        let (withdrawal_req1, req_info1) = withdrawal_request(&params, &sk_user1).unwrap();

        // ISSUE PARTIAL WALLETS for USER1
        let mut partial_wallets1 = Vec::new();
        for auth_keypair in authorities_keypairs.clone() {
            let blind_signature = issue(
                &params,
                &withdrawal_req1,
                pk_user1.clone(),
                &auth_keypair.secret_key(),
            ).unwrap();
            let partial_wallet1 = issue_verify(&grp, &auth_keypair.verification_key(), &sk_user1, &blind_signature, &req_info1).unwrap();
            partial_wallets1.push(partial_wallet1);
        }

        // AGGREGATE WALLET FOR USER1
        let mut wallet1 = aggregate_wallets(&grp, &verification_key, &sk_user1, &partial_wallets1).unwrap();

        let pay_info1 = PayInfo { info: [67u8; 32] };
        let (payment1, new_wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info1, 10, false).unwrap();

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = wallet1.l.get();
        wallet1.l.set(current_l - 7);

        let pay_info2 = PayInfo { info: [52u8; 32] };
        let (payment2, wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info2, 10, false).unwrap();


        let identify_result = identify(&params, &verification_key, &public_keys, payment1, payment2, pay_info1, pay_info2).unwrap();

        assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(pk_user1));
    }


    #[test]
    fn ok_if_two_different_payments() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        // KEY GENERATION FOR THE AUTHORITIES
        let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // KEY GENERATION FOR THE USER1
        let sk1 = grp.random_scalar();
        let sk_user1 = SecretKeyUser { sk: sk1 };
        let pk_user1 = SecretKeyUser::public_key(&sk_user1, &grp);

        // KEY GENERATION FOR THE USER2
        let sk2 = grp.random_scalar();
        let sk_user2 = SecretKeyUser { sk: sk2 };
        let pk_user2 = SecretKeyUser::public_key(&sk_user2, &grp);

        // WITHDRAWAL REQUEST FOR USER1
        let (withdrawal_req1, req_info1) = withdrawal_request(&params, &sk_user1).unwrap();

        // ISSUE PARTIAL WALLETS for USER1
        let mut partial_wallets1 = Vec::new();
        for auth_keypair in authorities_keypairs.clone() {
            let blind_signature = issue(
                &params,
                &withdrawal_req1,
                pk_user1.clone(),
                &auth_keypair.secret_key(),
            ).unwrap();
            let partial_wallet1 = issue_verify(&grp, &auth_keypair.verification_key(), &sk_user1, &blind_signature, &req_info1).unwrap();
            partial_wallets1.push(partial_wallet1);
        }

        // AGGREGATE WALLET FOR USER1
        let mut wallet1 = aggregate_wallets(&grp, &verification_key, &sk_user1, &partial_wallets1).unwrap();

        let pay_info1 = PayInfo { info: [67u8; 32] };
        let (payment1, wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info1, 10, false).unwrap();

        // SPEND VERIFICATION for USER1
        assert!(payment1.spend_verify(&params, &verification_key, &pay_info1).unwrap());

        // WITHDRAWAL REQUEST FOR USER2
        let (withdrawal_req2, req_info2) = withdrawal_request(&params, &sk_user2).unwrap();

        // ISSUE PARTIAL WALLETS for USER2
        let mut partial_wallets2 = Vec::new();
        for auth_keypair in authorities_keypairs.clone() {
            let blind_signature = issue(
                &params,
                &withdrawal_req2,
                pk_user2.clone(),
                &auth_keypair.secret_key(),
            ).unwrap();
            let partial_wallet2 = issue_verify(&grp, &auth_keypair.verification_key(), &sk_user2, &blind_signature, &req_info2).unwrap();
            partial_wallets2.push(partial_wallet2);
        }

        // AGGREGATE WALLET FOR USER2
        let mut wallet2 = aggregate_wallets(&grp, &verification_key, &sk_user2, &partial_wallets2).unwrap();

        let pay_info2 = PayInfo { info: [67u8; 32] };
        let (payment2, wallet2) = wallet2.spend(&params, &verification_key, &sk_user2, &pay_info2, 10, false).unwrap();

        // SPEND VERIFICATION for USER2
        assert!(payment2.spend_verify(&params, &verification_key, &pay_info2).unwrap());

        let public_keys = HashSet::from([pk_user1, pk_user2]);
        let identify_result = identify(&params, &verification_key, &public_keys, payment1, payment2, pay_info1, pay_info2).unwrap();
        assert_eq!(identify_result, IdentifyResult::NotADuplicatePayment);
    }
}