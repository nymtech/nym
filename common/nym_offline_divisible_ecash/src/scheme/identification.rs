use std::collections::HashMap;
use std::ops::Neg;

use bls12_381::{Gt, pairing};
use group::Curve;

use crate::error::{DivisibleEcashError, Result};
use crate::scheme::{PayInfo, Payment};
use crate::scheme::identification::IdentifyResult::DoubleSpendingPublicKeys;
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::Parameters;

pub enum IdentifyResult {
    NoCommonSerialNumbers,
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
        let sn = pairing(&payment1.phi.0.to_affine(), &params_a.get_ith_delta(k1 as usize).to_affine())
            + pairing(&payment1.phi.1.to_affine(), &params_a.get_ith_eta(k1 as usize).to_affine());
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
        Ok(IdentifyResult::NoCommonSerialNumbers)
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

    use crate::scheme::keygen::PublicKeyUser;
    use crate::scheme::setup::{GroupParameters, Parameters};
    use crate::utils::hash_g1;

    #[test]
    fn no_matching_serial_numbers() {}

    #[test]
    fn matching_payinfo() {}

    #[test]
    fn identified_duplicate_serial_number_and_non_matching_payinfo() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        let pk_u1 = PublicKeyUser { pk: hash_g1("PublicKey1") };
        let pk_u2 = PublicKeyUser { pk: hash_g1("PublicKey1") };
        let pk_u3 = PublicKeyUser { pk: hash_g1("PublicKey1") };
    }
}