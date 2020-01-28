use pemstore::pathfinder::PathFinder;
use std::path::PathBuf;

pub struct ClientPathfinder {
    pub config_dir: PathBuf,
    pub private_mix_key: PathBuf,
    pub public_mix_key: PathBuf,
}

impl ClientPathfinder {
    pub fn new(id: String) -> Self {
        let os_config_dir = dirs::config_dir().unwrap(); // grabs the OS default config dir
        let config_dir = os_config_dir.join("nym").join("clients").join(id);
        let private_mix_key = config_dir.join("private.pem");
        let public_mix_key = config_dir.join("public.pem");
        ClientPathfinder {
            config_dir,
            private_mix_key,
            public_mix_key,
        }
    }
}

impl PathFinder for ClientPathfinder {
    fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    fn private_identity_key(&self) -> PathBuf {
        self.private_mix_key.clone()
    }

    fn public_identity_key(&self) -> PathBuf {
        self.public_mix_key.clone()
    }
}
