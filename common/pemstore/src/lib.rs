// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use crate::traits::{PemStorableKey, PemStorableKeyPair};
use pem::{self, Pem};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub mod traits;

pub struct KeyPairPath {
    private_key_path: PathBuf,
    public_key_path: PathBuf,
}

impl KeyPairPath {
    pub fn new(private_key_path: PathBuf, public_key_path: PathBuf) -> Self {
        KeyPairPath {
            private_key_path,
            public_key_path,
        }
    }
}

pub fn load_keypair<T>(paths: &KeyPairPath) -> io::Result<T>
where
    T: PemStorableKeyPair,
{
    let private = load_key::<T::PrivatePemKey>(&paths.private_key_path)?;
    let public = load_key::<T::PublicPemKey>(&paths.public_key_path)?;
    Ok(T::from_keys(private, public))
}

pub fn store_keypair<T>(keypair: &T, paths: &KeyPairPath) -> io::Result<()>
where
    T: PemStorableKeyPair,
{
    store_key(keypair.public_key(), &paths.public_key_path)?;
    store_key(keypair.private_key(), &paths.private_key_path)
}

pub fn load_key<T>(path: &Path) -> io::Result<T>
where
    T: PemStorableKey,
{
    let key_pem = read_pem_file(path)?;

    if T::pem_type() != key_pem.tag {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "unexpected key pem tag",
        ));
    }

    let key = match T::from_bytes(&key_pem.contents) {
        Ok(key) => key,
        Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err.to_string())),
    };

    Ok(key)
}

pub fn store_key<T>(key: &T, path: &Path) -> io::Result<()>
where
    T: PemStorableKey,
{
    write_pem_file(path, key.to_bytes(), T::pem_type())
}

fn read_pem_file(filepath: &Path) -> io::Result<Pem> {
    let mut pem_bytes = File::open(filepath)?;
    let mut buf = Vec::new();
    pem_bytes.read_to_end(&mut buf)?;
    pem::parse(&buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

fn write_pem_file(filepath: &Path, data: Vec<u8>, tag: &str) -> io::Result<()> {
    // ensure the whole directory structure exists
    if let Some(parent_dir) = filepath.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }
    let pem = Pem {
        tag: tag.to_string(),
        contents: data,
    };
    let key = pem::encode(&pem);

    let mut file = File::create(filepath)?;
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
