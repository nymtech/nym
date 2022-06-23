use std::collections::HashMap;
use std::ops::Neg;

use bls12_381::{Gt, pairing};
use group::Curve;

use crate::error::{DivisibleEcashError, Result};
use crate::scheme::{PayInfo, Payment};
use crate::scheme::identification::IdentifyResult::DoubleSpendingPublicKeys;
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::Parameters;

#[derive(Debug, Eq, PartialEq)]
pub enum IdentifyResult {
    NotADuplicatePayment,
    DuplicatePayInfo(PayInfo),
    DoubleSpendingPublicKeys(Vec<PublicKeyUser>),
    Whatever,
}

// how do we get the list of all pkU ?
pub fn identify(
    params: &Parameters,
    public_keys_u: &[PublicKeyUser],
    payment1: Payment,
    payment2: Payment,
    payinfo1: PayInfo,
    payinfo2: PayInfo) -> Result<IdentifyResult> {
    let params_a = params.get_params_a();
    // compute the serial numbers for k1 in [0, V1-1]
    let mut serial_numbers = HashMap::new();
    for k1 in 0..payment1.vv {
        let pg1 = pairing(&payment1.phi.0.to_affine(), &params_a.get_ith_delta(k1 as usize).to_affine());
        let pg2 = pairing(&payment1.phi.1.to_affine(), &params_a.get_ith_eta(k1 as usize).to_affine());
        let sn = pg1 + pg2;
        serial_numbers.insert(sn, k1);
    }

    // compute the serial numbers fo k2 in [0, V2-1]
    let mut duplicate_serial_numbers: Vec<(Gt, u64, u64)> = Default::default();
    for k2 in 0..payment2.vv {
        let sn = pairing(&payment2.phi.0.to_affine(), &params_a.get_ith_delta(k2 as usize).to_affine())
            + pairing(&payment2.phi.1.to_affine(), &params_a.get_ith_eta(k2 as usize).to_affine());
        if !serial_numbers.contains_key(&sn) {
            serial_numbers.insert(sn, k2);
        } else {
            let k1 = *serial_numbers.get(&sn).unwrap() as u64;
            duplicate_serial_numbers.push((sn, k1, k2));
        }
    }

    if duplicate_serial_numbers.is_empty() {
        Ok(IdentifyResult::NotADuplicatePayment)
    } else {
        if payinfo1.info == payinfo2.info {
            Ok(IdentifyResult::DuplicatePayInfo(payinfo1))
        } else {
            let mut identified_pk_u: Vec<PublicKeyUser> = Default::default();
            for elem in duplicate_serial_numbers.iter() {
                let k1 = elem.1;
                let k2 = elem.2;
                let delta_k1 = params_a.get_ith_delta(k1 as usize);
                let delta_k2 = params_a.get_ith_delta(k2 as usize);
                let tt1 = pairing(&payment1.varphi.1.to_affine(), &delta_k1.to_affine())
                    + pairing(&payment1.varphi.0.to_affine(), &params_a.get_ith_eta(k1 as usize).to_affine());
                let tt2 = pairing(&payment2.varphi.1.to_affine(), &delta_k2.to_affine())
                    + pairing(&payment2.varphi.0.to_affine(), &params_a.get_ith_eta(k2 as usize).to_affine());

                for pk_u in public_keys_u.iter() {
                    let pg_pku_deltas = pairing(&pk_u.pk.to_affine(), &(delta_k1 * payment1.rr.neg() + delta_k2 * payment2.rr.neg()).to_affine());
                    if tt1 + tt2.neg() == pg_pku_deltas {
                        identified_pk_u.push(pk_u.clone());
                    }
                }
            }
            if !identified_pk_u.is_empty() {
                Ok(DoubleSpendingPublicKeys(identified_pk_u.clone()))
            } else {
                return Err(DivisibleEcashError::Identify(
                    "A duplicate serial number was detected, the payinfo1 and payinfo2 are different, but we failed to identify the double-spending public key".to_string(),
                ));
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use crate::scheme::{PayInfo, Payment};
    use crate::scheme::aggregation::{aggregate_verification_keys, aggregate_wallets};
    use crate::scheme::identification::{identify, IdentifyResult};
    use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser, ttp_keygen_authorities, VerificationKeyAuth};
    use crate::scheme::setup::{GroupParameters, Parameters};
    use crate::scheme::withdrawal::{issue, issue_verify, withdrawal_request};
    use crate::utils::hash_g1;

    #[test]
    fn no_matching_serial_numbers() {}

    #[test]
    fn matching_payinfo() {}

    #[test]
    fn identified_duplicate_serial_number_and_non_matching_pay_info() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        let pk_u1 = PublicKeyUser { pk: hash_g1("PublicKey1") };
        let pk_u2 = PublicKeyUser { pk: hash_g1("PublicKey1") };
        let pk_u3 = PublicKeyUser { pk: hash_g1("PublicKey1") };
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
        let (payment1, wallet1) = wallet1.spend(&params, &verification_key, &sk_user1, &pay_info1, 10).unwrap();

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
        let (payment2, wallet2) = wallet2.spend(&params, &verification_key, &sk_user2, &pay_info2, 10).unwrap();

        // SPEND VERIFICATION for USER2
        assert!(payment2.spend_verify(&params, &verification_key, &pay_info2).unwrap());

        let identify_result = identify(&params, &[pk_user1, pk_user2], payment1, payment2, pay_info1, pay_info2).unwrap();
        assert_eq!(identify_result, IdentifyResult::NotADuplicatePayment);
    }
}