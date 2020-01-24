use crate::filter;

#[derive(Debug, Clone)]
pub struct Node {
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl filter::Versioned for Node {
    fn get_version(&self) -> String {
        self.version.clone()
    }
}
