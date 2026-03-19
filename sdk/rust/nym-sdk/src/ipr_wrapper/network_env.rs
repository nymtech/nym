use std::fs;
use std::path::PathBuf;

use crate::Error;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEnvironment {
    #[default]
    Mainnet,
}

fn find_workspace_root() -> Result<PathBuf, Error> {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    loop {
        let cargo_toml = current.join("Cargo.toml");

        if cargo_toml.exists() {
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    return Ok(current);
                }
            }
        }

        if !current.pop() {
            return Err(Error::WorkspaceRootNotFound);
        }
    }
}

impl NetworkEnvironment {
    pub fn env_file_path(&self) -> Result<PathBuf, Error> {
        let root = find_workspace_root()?;
        match self {
            Self::Mainnet => Ok(root.join("envs/mainnet.env")),
        }
    }

    pub fn network_defaults(&self) -> crate::NymNetworkDetails {
        match self {
            Self::Mainnet => crate::NymNetworkDetails::new_mainnet(),
        }
    }
}
