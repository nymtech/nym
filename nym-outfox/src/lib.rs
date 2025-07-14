pub mod constants;
pub mod error;
pub mod format;
pub mod lion;
pub mod packet;
pub mod route;

#[cfg(test)]
mod test {
    use libcrux_kem::*;
    use rand::rngs::OsRng;
    use rand::TryRngCore;

    #[test]
    fn test_kem() {
        let mut os_rng = OsRng;
        let mut rng = os_rng.unwrap_mut();

        let (sk_a, pk_a) = key_gen(Algorithm::MlKem768, &mut rng).unwrap();

        let received_sk = sk_a.encode();
        let received_pk = pk_a.encode();

        let pk = PublicKey::decode(Algorithm::MlKem768, &received_pk).unwrap();
        let (ss_b, ct_b) = pk.encapsulate(&mut rng).unwrap();
        let received_ct = ct_b.encode();

        println!("pk: {}", received_pk.len());
        println!("sk: {}", received_sk.len());
        println!("kem encaps: {}", received_ct.len());

        let ct_a = Ct::decode(Algorithm::MlKem768, &received_ct).unwrap();
        let ss_a = ct_a.decapsulate(&sk_a).unwrap();
        assert_eq!(ss_b.encode(), ss_a.encode());
    }
}
