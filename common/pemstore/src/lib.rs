// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::traits::{PemStorableKey, PemStorableKeyPair};
use pem::Pem;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tracing::debug;
use zeroize::{Zeroize, Zeroizing};

#[cfg(feature = "encryption")]
pub mod encryption;
pub mod traits;

#[cfg(feature = "encryption")]
pub use encryption::Passphrase;

/// Prefix on the PEM tag of a private key encrypted with a passphrase.
const ENCRYPTED_TAG_PREFIX: &str = "ENCRYPTED ";

const PEM_BEGIN: &str = "-----BEGIN ";

struct ZeroizingPem(Pem);

impl Zeroize for ZeroizingPem {
    fn zeroize(&mut self) {
        self.0.tag.zeroize();
        self.0.contents.zeroize();
    }
}
impl Drop for ZeroizingPem {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Deref for ZeroizingPem {
    type Target = Pem;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct KeyPairPath {
    pub private_key_path: PathBuf,
    pub public_key_path: PathBuf,
}

impl KeyPairPath {
    pub fn new<P: AsRef<Path>>(private_key_path: P, public_key_path: P) -> Self {
        KeyPairPath {
            private_key_path: private_key_path.as_ref().to_owned(),
            public_key_path: public_key_path.as_ref().to_owned(),
        }
    }
}

pub fn load_keypair<T>(paths: &KeyPairPath) -> io::Result<T>
where
    T: PemStorableKeyPair,
{
    let private: T::PrivatePemKey = load_key(&paths.private_key_path)?;
    let public: T::PublicPemKey = load_key(&paths.public_key_path)?;
    Ok(T::from_keys(private, public))
}

/// Loads a keypair whose private key may be passphrase-encrypted; the public key never is.
#[cfg(feature = "encryption")]
pub fn load_keypair_with<T>(paths: &KeyPairPath, passphrase: Option<&Passphrase>) -> io::Result<T>
where
    T: PemStorableKeyPair,
{
    let private: T::PrivatePemKey = load_key_with(&paths.private_key_path, passphrase)?;
    let public: T::PublicPemKey = load_key(&paths.public_key_path)?;
    Ok(T::from_keys(private, public))
}

pub fn store_keypair<T>(keypair: &T, paths: &KeyPairPath) -> io::Result<()>
where
    T: PemStorableKeyPair,
{
    store_key(keypair.public_key(), &paths.public_key_path)?;
    store_key(keypair.private_key(), &paths.private_key_path)
}

/// Stores a keypair, encrypting the private key when a passphrase is given; the public key never is.
#[cfg(feature = "encryption")]
pub fn store_keypair_with<T>(
    keypair: &T,
    paths: &KeyPairPath,
    passphrase: Option<&Passphrase>,
) -> io::Result<()>
where
    T: PemStorableKeyPair,
{
    store_key(keypair.public_key(), &paths.public_key_path)?;
    store_key_with(keypair.private_key(), &paths.private_key_path, passphrase)
}

pub fn load_key<T, P>(path: P) -> io::Result<T>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    debug!(
        "attempting to load key with the following pem type: {}",
        T::pem_type()
    );
    let key_pem = read_pem_file(&path)?;
    decode_key(&key_pem, path.as_ref())
}

/// Loads a key from its PEM file, decrypting it when the file is encrypted and a passphrase is given.
#[cfg(feature = "encryption")]
pub fn load_key_with<T, P>(path: P, passphrase: Option<&Passphrase>) -> io::Result<T>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    let Some(passphrase) = passphrase else {
        return load_key(path);
    };

    debug!(
        "attempting to load key with the following pem type: {}",
        T::pem_type()
    );
    let key_pem = read_pem_file(&path)?;
    if key_pem.tag != encrypted_tag::<T>() {
        return decode_key(&key_pem, path.as_ref());
    }

    let plaintext = encryption::decrypt(passphrase, &key_pem.contents)?;
    T::from_bytes(&plaintext).map_err(invalid_data)
}

pub fn store_key<T, P>(key: &T, path: P) -> io::Result<()>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    write_pem_file(path, key.to_bytes(), T::pem_type())
}

/// Stores a key to its PEM file, encrypting it when a passphrase is given.
#[cfg(feature = "encryption")]
pub fn store_key_with<T, P>(key: &T, path: P, passphrase: Option<&Passphrase>) -> io::Result<()>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    let Some(passphrase) = passphrase else {
        return store_key(key, path);
    };

    let plaintext = Zeroizing::new(key.to_bytes());
    let ciphertext = encryption::encrypt(passphrase, &plaintext)?;
    write_pem_file(path, ciphertext, &encrypted_tag::<T>())
}

/// Whether the PEM file at `path` holds a passphrase-encrypted key, judged from its header alone.
pub fn is_encrypted<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let encrypted_begin = format!("{PEM_BEGIN}{ENCRYPTED_TAG_PREFIX}");

    // the shortest plaintext header we ever write is longer than this, so the read
    // stops before any key material
    let mut header = vec![0u8; encrypted_begin.len()];
    File::open(path)?.read_exact(&mut header)?;

    if header.starts_with(encrypted_begin.as_bytes()) {
        Ok(true)
    } else if header.starts_with(PEM_BEGIN.as_bytes()) {
        Ok(false)
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "not a pem file"))
    }
}

/// Decodes a plaintext key; an encrypted one is reported as needing a passphrase.
fn decode_key<T: PemStorableKey>(key_pem: &ZeroizingPem, path: &Path) -> io::Result<T> {
    if key_pem.tag == T::pem_type() {
        T::from_bytes(&key_pem.contents).map_err(invalid_data)
    } else if key_pem.tag == encrypted_tag::<T>() {
        Err(io::Error::other(format!(
            "the key at '{}' is encrypted and requires a passphrase",
            path.display()
        )))
    } else {
        Err(io::Error::other(format!(
            "unexpected key pem tag. Got '{}', expected: '{}'",
            key_pem.tag,
            T::pem_type()
        )))
    }
}

fn invalid_data(err: impl std::error::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

fn encrypted_tag<T: PemStorableKey>() -> String {
    format!("{ENCRYPTED_TAG_PREFIX}{}", T::pem_type())
}

fn read_pem_file<P: AsRef<Path>>(filepath: P) -> io::Result<ZeroizingPem> {
    let mut pem_bytes = File::open(filepath)?;
    let mut buf = Zeroizing::new(Vec::new());
    pem_bytes.read_to_end(&mut buf)?;
    pem::parse(&buf).map(ZeroizingPem).map_err(io::Error::other)
}

fn write_pem_file<P: AsRef<Path>>(filepath: P, data: Vec<u8>, tag: &str) -> io::Result<()> {
    let filepath = filepath.as_ref();

    // use Zeroizing wrappers to ensure value is zeroed on any possible failure down the line
    let pem = ZeroizingPem(Pem {
        tag: tag.to_string(),
        contents: data,
    });
    let encoded = Zeroizing::new(pem::encode(&pem));

    // ensure the whole directory structure exists
    if let Some(parent_dir) = filepath.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    // write it into a temp file and rename it over the target after.
    // this is so that an interrupted write could never leave a truncated key behind
    let tmp_path = tmp_path(filepath);
    let mut file = create_owner_only_file(&tmp_path)?;
    if let Err(err) = file
        .write_all(encoded.as_bytes())
        .and_then(|_| file.sync_all())
    {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }
    #[cfg(not(target_arch = "wasm32"))]
    drop(file);

    if let Err(err) = fs::rename(&tmp_path, filepath) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    Ok(())
}

/// The temp file a key is written to before being renamed into place.
fn tmp_path(filepath: &Path) -> PathBuf {
    let mut tmp = filepath.as_os_str().to_owned();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

/// Creates (or truncates) `path` with owner-only permissions; every key file, public ones included, gets them.
fn create_owner_only_file(path: &Path) -> io::Result<File> {
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;

        // `mode` only applies to a newly created file; a stale one keeps whatever it had
        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;

        Ok(file)
    }

    // note: there is no equivalent of unix file modes on other systems, like Windows.
    // TODO: a possible consideration would be to use `permission.set_readonly(true)`,
    // which would work on both platforms, but that would leave keys on unix with 0444,
    // which I feel is too open.
    #[cfg(not(target_family = "unix"))]
    {
        File::create(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DummyKey([u8; 32]);

    impl PemStorableKey for DummyKey {
        type Error = std::array::TryFromSliceError;

        fn pem_type() -> &'static str {
            "DUMMY KEY"
        }

        fn to_bytes(&self) -> Vec<u8> {
            self.0.to_vec()
        }

        fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
            bytes.try_into().map(DummyKey)
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct DummyKeyPair {
        private: DummyKey,
        public: DummyKey,
    }

    impl PemStorableKeyPair for DummyKeyPair {
        type PrivatePemKey = DummyKey;
        type PublicPemKey = DummyKey;

        fn private_key(&self) -> &DummyKey {
            &self.private
        }

        fn public_key(&self) -> &DummyKey {
            &self.public
        }

        fn from_keys(private: DummyKey, public: DummyKey) -> Self {
            DummyKeyPair { private, public }
        }
    }

    fn key_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("keys").join("dummy.pem")
    }

    #[test]
    fn store_then_load_key_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_path(&dir);

        store_key(&DummyKey([1; 32]), &path).unwrap();

        assert_eq!(load_key::<DummyKey, _>(&path).unwrap(), DummyKey([1; 32]));
        assert!(!tmp_path(&path).exists());
    }

    #[test]
    fn store_key_replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_path(&dir);

        store_key(&DummyKey([1; 32]), &path).unwrap();
        store_key(&DummyKey([2; 32]), &path).unwrap();

        assert_eq!(load_key::<DummyKey, _>(&path).unwrap(), DummyKey([2; 32]));
        assert!(!tmp_path(&path).exists());
    }

    #[test]
    fn store_key_overwrites_a_stale_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(tmp_path(&path), "leftover from an interrupted write").unwrap();

        store_key(&DummyKey([3; 32]), &path).unwrap();

        assert_eq!(load_key::<DummyKey, _>(&path).unwrap(), DummyKey([3; 32]));
        assert!(!tmp_path(&path).exists());
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn store_then_load_keypair_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let paths = KeyPairPath::new(
            dir.path().join("private.pem"),
            dir.path().join("public.pem"),
        );
        let keypair = DummyKeyPair {
            private: DummyKey([8; 32]),
            public: DummyKey([9; 32]),
        };

        store_keypair(&keypair, &paths).unwrap();

        assert_eq!(load_keypair::<DummyKeyPair>(&paths).unwrap(), keypair);
    }

    #[test]
    fn load_key_rejects_a_wrong_tag() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_path(&dir);
        write_pem_file(&path, vec![1; 32], "OTHER KEY").unwrap();

        let err = load_key::<DummyKey, _>(&path).unwrap_err();

        assert!(err.to_string().contains("unexpected key pem tag"), "{err}");
    }

    #[test]
    fn load_key_without_passphrase_rejects_an_encrypted_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_path(&dir);
        write_pem_file(&path, vec![0; 8], &encrypted_tag::<DummyKey>()).unwrap();

        let err = load_key::<DummyKey, _>(&path).unwrap_err();

        assert!(err.to_string().contains("requires a passphrase"), "{err}");
    }

    #[test]
    fn is_encrypted_reads_the_header() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain.pem");
        let encrypted = dir.path().join("encrypted.pem");
        let garbage = dir.path().join("garbage");
        let empty = dir.path().join("empty");
        write_pem_file(&plain, vec![1; 32], DummyKey::pem_type()).unwrap();
        write_pem_file(&encrypted, vec![0; 8], &encrypted_tag::<DummyKey>()).unwrap();
        std::fs::write(&garbage, "definitely not a pem file at all").unwrap();
        std::fs::write(&empty, "").unwrap();

        assert!(!is_encrypted(&plain).unwrap());
        assert!(is_encrypted(&encrypted).unwrap());
        assert!(is_encrypted(&garbage).is_err());
        assert!(is_encrypted(&empty).is_err());
        assert!(is_encrypted(dir.path().join("missing")).is_err());
    }

    #[cfg(feature = "encryption")]
    mod with_encryption {
        use super::*;

        #[test]
        fn store_key_with_passphrase_round_trips() {
            let dir = tempfile::tempdir().unwrap();
            let path = key_path(&dir);
            let passphrase = Passphrase::new("hunter2");

            store_key_with(&DummyKey([4; 32]), &path, Some(&passphrase)).unwrap();

            assert!(is_encrypted(&path).unwrap());
            assert_eq!(
                load_key_with::<DummyKey, _>(&path, Some(&passphrase)).unwrap(),
                DummyKey([4; 32])
            );
            assert!(load_key_with::<DummyKey, _>(&path, None).is_err());
            assert!(
                load_key_with::<DummyKey, _>(&path, Some(&Passphrase::new("hunter3"))).is_err()
            );
        }

        #[test]
        fn load_key_with_passphrase_accepts_a_plaintext_key() {
            let dir = tempfile::tempdir().unwrap();
            let path = key_path(&dir);
            store_key(&DummyKey([5; 32]), &path).unwrap();

            let loaded = load_key_with::<DummyKey, _>(&path, Some(&Passphrase::new("hunter2")));

            assert_eq!(loaded.unwrap(), DummyKey([5; 32]));
            assert!(
                !is_encrypted(&path).unwrap(),
                "loading must never rewrite the file"
            );
        }

        #[test]
        fn store_keypair_with_passphrase_encrypts_only_the_private_key() {
            let dir = tempfile::tempdir().unwrap();
            let paths = KeyPairPath::new(
                dir.path().join("private.pem"),
                dir.path().join("public.pem"),
            );
            let passphrase = Passphrase::new("hunter2");
            let keypair = DummyKeyPair {
                private: DummyKey([6; 32]),
                public: DummyKey([7; 32]),
            };

            store_keypair_with(&keypair, &paths, Some(&passphrase)).unwrap();

            assert!(is_encrypted(&paths.private_key_path).unwrap());
            assert!(!is_encrypted(&paths.public_key_path).unwrap());
            assert_eq!(
                load_keypair_with::<DummyKeyPair>(&paths, Some(&passphrase)).unwrap(),
                keypair
            );
            assert!(load_keypair::<DummyKeyPair>(&paths).is_err());
        }
    }
}
