use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeDescription {
    name: String,
    description: String,
    link: String,
}

impl Default for NodeDescription {
    fn default() -> Self {
        NodeDescription {
            name: "This node has not yet set a name".to_string(),
            description: "This node has not yet set a description".to_string(),
            link: "https://nymtech.net".to_string(),
        }
    }
}

impl NodeDescription {
    pub(crate) fn load_from_file(mut config_path: PathBuf) -> io::Result<NodeDescription> {
        config_path.push("descriptor.json");
        let json = fs::read_to_string(config_path)?;
        ::serde_json::from_str(&json)
            .map_err(|json_err| io::Error::new(io::ErrorKind::Other, json_err))
    }
}
