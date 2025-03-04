#[cfg(test)]
mod tests {
    use crate::client::key_manager::ClientKeys;
    use nym_crypto::hkdf::DerivationMaterial;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_from_master_key_success() {
        // Set up a deterministic RNG.
        let seed = [33u8; 32];
        let mut rng = ChaCha20Rng::from_seed(seed);

        // Set up the derivation material.
        let master_key = b"this is a secret master key";
        let salt = b"unique-salt";
        let derivation_material = DerivationMaterial::new(master_key, 0, salt);

        // Generate ClientKeys from the master key.
        let client_keys = ClientKeys::from_master_key(&mut rng, &derivation_material)
            .expect("Failed to create client keys");

        assert_eq!(
            client_keys.identity_keypair().public_key().to_string(),
            String::from("FX4Undr5LPPBA7zThWWpAKXKQTXSbW1C28PnxbCqUkU4")
        );

        assert_eq!(
            client_keys.identity_keypair().private_key().to_string(),
            String::from("6S3uMi2rU5SwyUUYCiMrF5qqdcYnEDMYLggBSvavVzEt")
        );
    }

    #[test]
    fn test_from_master_key_deterministic_identity() {
        // Using identical derivation material should result in the exactly same identity keypair.
        let seed = [1u8; 32];
        let mut rng1 = ChaCha20Rng::from_seed(seed);
        let mut rng2 = ChaCha20Rng::from_seed(seed);

        let master_key = b"another secret master key";
        let salt = b"deterministic-salt";
        let index = 7u32;
        let derivation_material = DerivationMaterial::new(master_key, index, salt);

        let client_keys1 = ClientKeys::from_master_key(&mut rng1, &derivation_material)
            .expect("Failed to create client keys (first instance)");
        let client_keys2 = ClientKeys::from_master_key(&mut rng2, &derivation_material)
            .expect("Failed to create client keys (second instance)");

        assert_eq!(
            client_keys1.identity_keypair().public_key().to_string(),
            client_keys2.identity_keypair().public_key().to_string()
        );

        assert_eq!(
            client_keys1.identity_keypair().private_key().to_string(),
            client_keys2.identity_keypair().private_key().to_string()
        );
    }

    #[test]
    fn test_from_master_key_different_indices() {
        // Changing the index should yield a different identity key.
        let seed = [5u8; 32];
        let mut rng = ChaCha20Rng::from_seed(seed);

        let master_key = b"same secret key";
        let salt = b"same-salt";

        let derivation_material1 = DerivationMaterial::new(master_key, 1, salt);
        let derivation_material2 = DerivationMaterial::new(master_key, 2, salt);

        let client_keys1 = ClientKeys::from_master_key(&mut rng, &derivation_material1)
            .expect("Failed to create client keys for index 1");
        let client_keys2 = ClientKeys::from_master_key(&mut rng, &derivation_material2)
            .expect("Failed to create client keys for index 2");

        assert_ne!(
            client_keys1.identity_keypair().public_key().to_string(),
            client_keys2.identity_keypair().public_key().to_string()
        );

        assert_ne!(
            client_keys1.identity_keypair().private_key().to_string(),
            client_keys2.identity_keypair().private_key().to_string()
        );
    }
}
