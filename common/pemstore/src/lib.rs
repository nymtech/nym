// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::traits::{PemStorableKey, PemStorableKeyPair};
use pem::Pem;
use std::fs::File;
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
    pem::parse(&buf)
        .map(ZeroizingPem)
        .map_err(io::Error::other)
}

fn write_pem_file<P: AsRef<Path>>(filepath: P, mut data: Vec<u8>, tag: &str) -> io::Result<()> {
    // ensure the whole directory structure exists
    if let Some(parent_dir) = filepath.as_ref().parent() {
        if let Err(err) = std::fs::create_dir_all(parent_dir) {
            // in case of a failure, make sure to zeroize the data before returning
            // (we can't wrap it in `Zeroize` due to `Pem` requirements)
            data.zeroize();
            return Err(err);
        }
    }

    let mut file = File::create(filepath.as_ref())?;

    let pem = ZeroizingPem(Pem {
        tag: tag.to_string(),
        contents: data,
    });
    let key = Zeroizing::new(pem::encode(&pem));
    file.write_all(key.as_bytes())?;

    // note: this is only supported on unix (on different systems, like Windows, it will just
    // be ignored)
    // TODO: a possible consideration would be to use `permission.set_readonly(true)`,
    // which would work on both platforms, but that would leave keys on unix with 0444,
    // which I feel is too open.
    #[cfg(target_family = "unix")]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(filepath, permissions)?;
    }

    Ok(())
}
