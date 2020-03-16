use crate::filter;

#[derive(Debug, Clone)]
pub struct Node {
    pub location: String,
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        self.version.clone()
    }
}
