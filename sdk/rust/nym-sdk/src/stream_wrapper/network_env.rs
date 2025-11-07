use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEnvironment {
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

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "mainnet" | "main" => Ok(Self::Mainnet),
            // "sandbox" | "sand" => Ok(Self::Sandbox),
            _ => Err(format!("Unknown env: {}", s)),
        }
    }
}

impl Default for NetworkEnvironment {
    fn default() -> Self {
        Self::Mainnet
    }
}
