use std::path::PathBuf;

pub trait PathFinder {
    fn config_dir(&self) -> PathBuf;
    fn private_identity_key(&self) -> PathBuf;
    fn public_identity_key(&self) -> PathBuf;

    // Optional:
    fn private_encryption_key(&self) -> Option<PathBuf> {
        None
    }
    fn public_encryption_key(&self) -> Option<PathBuf> {
        None
    }
}
