use std::{fs::File, io::Write, path::Path};

use tracing::info;

pub(crate) fn generate_key_pair(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let priv_key_path = path.as_ref();
    let mut rng = rand::thread_rng();
    let keypair = nym_crypto::asymmetric::identity::KeyPair::new(&mut rng);
    info!("Generated keypair as Base58-encoded string");

    let mut private_key_file = File::create(priv_key_path)?;
    private_key_file.write_all(keypair.private_key().to_base58_string().as_bytes())?;

    let pub_key_path = priv_key_path.with_extension("public");
    let mut public_key_file = File::create(&pub_key_path)?;
    public_key_file.write_all(keypair.public_key().to_base58_string().as_bytes())?;

    info!(
        "Saved Base58-encoded keypair, private key to {}, public key to {}",
        priv_key_path.display(),
        pub_key_path.display()
    );
    info!("Public key should be whitelisted with NS API");

    Ok(())
}

#[cfg(test)]
mod test {
    use nym_crypto::asymmetric::ed25519::PrivateKey;
    use tempfile::TempDir;

    use super::*;

    use std::{
        fs::{self},
        path::PathBuf,
    };

    #[test]
    fn can_generate_valid_keypair() {
        let tmp_dir = TempDir::new().unwrap();
        let pkey_file = PathBuf::from_iter(&[
            tmp_dir.path().to_path_buf(),
            PathBuf::from("agent-key-private"),
        ]);
        generate_key_pair(&pkey_file).expect("Failed to generate keypair");

        let pkey_raw = fs::read_to_string(&pkey_file).expect("Failed to read file");
        let key = PrivateKey::from_base58_string(pkey_raw).expect("Failed to load key");

        let msg = "hello, world";

        let signature = key.sign(msg);
        key.public_key()
            .verify(msg, &signature)
            .expect("Failed to verify signature");
    }
}
