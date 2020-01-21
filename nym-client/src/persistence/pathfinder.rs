use std::path::PathBuf;

pub struct Pathfinder {
    pub config_dir: PathBuf,
    pub private_mix_key: PathBuf,
    pub public_mix_key: PathBuf,
}

impl Pathfinder {
    pub fn new(id: String) -> Pathfinder {
        let os_config_dir = dirs::config_dir().unwrap(); // grabs the OS default config dir
        let config_dir = os_config_dir.join("nym").join("client").join(id);
        let private_mix_key = config_dir.join("private.pem");
        let public_mix_key = config_dir.join("public.pem");
        Pathfinder {
            config_dir,
            private_mix_key,
            public_mix_key,
        }
    }
}
