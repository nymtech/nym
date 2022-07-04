use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::{fs, io};

pub(crate) const DESCRIPTION_FILE: &str = "description.toml";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NodeDescription {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) link: String,
    pub(crate) location: String,
}

impl Default for NodeDescription {
    fn default() -> Self {
        NodeDescription {
            name: "This node has not yet set a name".to_string(),
            description: "This node has not yet set a description".to_string(),
            link: "https://nymtech.net".to_string(),
            location: "This node has not yet set a location".to_string(),
        }
    }
}

impl NodeDescription {
    pub(crate) fn load_from_file(config_path: PathBuf) -> io::Result<NodeDescription> {
        let description_file_path: PathBuf = [config_path.to_str().unwrap(), DESCRIPTION_FILE]
            .iter()
            .collect();
        let toml = fs::read_to_string(description_file_path)?;
        toml::from_str(&toml).map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
    }

    pub(crate) fn save_to_file(
        description: &NodeDescription,
        config_path: PathBuf,
    ) -> io::Result<()> {
        let description_file_path: PathBuf = [config_path.to_str().unwrap(), DESCRIPTION_FILE]
            .iter()
            .collect();
        let description_toml =
            toml::to_string(description).expect("could not encode description to toml");
        fs::write(description_file_path, description_toml)?;
        Ok(())
    }
}
