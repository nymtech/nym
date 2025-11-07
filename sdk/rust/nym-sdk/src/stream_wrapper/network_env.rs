use std::path::PathBuf;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEnvironment {
    #[default]
    Mainnet,
    // Sandbox,
}

impl NetworkEnvironment {
    pub fn env_file_path(&self) -> PathBuf {
        match self {
            Self::Mainnet => PathBuf::from("../../../envs/sandbox.env"),
            // Self::Sandbox => PathBuf::from("../../../envs/sandbox.env"),
        }
    }

    pub fn network_defaults(&self) -> crate::NymNetworkDetails {
        match self {
            Self::Mainnet => crate::NymNetworkDetails::new_mainnet(),
            // Self::Sandbox => crate::NymNetworkDetails::new_sandbox(), // TODO make this fn
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
