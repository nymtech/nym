use std::fs;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEnvironment {
    #[default]
    Mainnet,
    // Sandbox,
}

fn find_workspace_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    loop {
        let cargo_toml = current.join("Cargo.toml");

        if cargo_toml.exists() {
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                // Check if this Cargo.toml defines a workspace
                if contents.contains("[workspace]") {
                    return current;
                }
            }
        }

        if !current.pop() {
            panic!("Could not find workspace root");
        }
    }
}

impl NetworkEnvironment {
    pub fn env_file_path(&self) -> PathBuf {
        let root = find_workspace_root();
        match self {
            Self::Mainnet => PathBuf::from(root.join("envs/mainnet.env")),
            // Self::Sandbox => PathBuf::from(root.join("envs/sandbox.env")),
        }
    }

    pub fn network_defaults(&self) -> crate::NymNetworkDetails {
        match self {
            Self::Mainnet => crate::NymNetworkDetails::new_mainnet(),
            // Self::Sandbox => crate::NymNetworkDetails::new_sandbox(), // TODO
        }
    }

    pub fn parse_network(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "mainnet" | "main" => Ok(Self::Mainnet),
            // "sandbox" | "sand" => Ok(Self::Sandbox),
            _ => Err(format!("Unknown env: {}", s)),
        }
    }
}
