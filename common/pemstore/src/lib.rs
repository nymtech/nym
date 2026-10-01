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

pub mod traits;

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

pub fn store_keypair<T>(keypair: &T, paths: &KeyPairPath) -> io::Result<()>
where
    T: PemStorableKeyPair,
{
    store_key(keypair.public_key(), &paths.public_key_path)?;
    store_key(keypair.private_key(), &paths.private_key_path)
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
    let key_pem = read_pem_file(path)?;

    if T::pem_type() != key_pem.tag {
        return Err(io::Error::other(format!(
            "unexpected key pem tag. Got '{}', expected: '{}'",
            key_pem.0.tag,
            T::pem_type()
        )));
    }

    let key = match T::from_bytes(&key_pem.contents) {
        Ok(key) => key,
        Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err.to_string())),
    };

    Ok(key)
}

pub fn store_key<T, P>(key: &T, path: P) -> io::Result<()>
where
    T: PemStorableKey,
    P: AsRef<Path>,
{
    write_pem_file(path, key.to_bytes(), T::pem_type())
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

    // only the keypair tests construct it, and they arrive with the passphrase-aware loaders
    #[allow(dead_code)]
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
}
