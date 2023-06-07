use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::{fs, io};

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
    pub(crate) fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<NodeDescription> {
        // let description_file_path: PathBuf = [config_path.to_str().unwrap(), DESCRIPTION_FILE]
        //     .iter()
        //     .collect();
        // let toml = fs::read_to_string(description_file_path)?;
        // toml::from_str(&toml).map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
        let toml = fs::read_to_string(path)?;
        toml::from_str(&toml).map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
    }

    pub(crate) fn save_to_file<P: AsRef<Path>>(
        description: &NodeDescription,
        path: P,
    ) -> io::Result<()> {
        // let description_file_path: PathBuf = [config_path.to_str().unwrap(), DESCRIPTION_FILE]
        //     .iter()
        //     .collect();
        let description_toml =
            toml::to_string(description).expect("could not encode description to toml");
        fs::write(path, description_toml)?;
        Ok(())
    }
}
