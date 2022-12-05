use std::path::PathBuf;

pub struct Client {
    pub nym_address: String,
    config: Config,
}

impl Client {
    /// Create a new mixnet client. If no config options are supplied, creates a new client with ephemeral keys
    /// stored in RAM, which will be discarded at application close.
    ///
    /// Callers have the option of supplying futher parameters to store persistent identities at a location on-disk,
    /// if desired.
    pub fn new(config_option: Option<Config>) -> Client {
        if config_option.is_none() {
            let config = Self::some_minimal_config();
            let nym_address = Self::new_nym_address();
            Client {
                config,
                nym_address,
            }
        } else {
            let self_address = Self::new_nym_address();
            let nym_address = Self::new_nym_address();

            Client {
                config: config_option.unwrap(),
                nym_address,
            }
        }
    }

    /// Connects to the mixnet via the gateway in the client config
    pub fn connect_to_mixnet(&self) {}

    /// Sets the callback function which is run when the client receives a message
    /// from the mixnet
    pub fn on_receive(&mut self, message: &str) {
        println!("Message received: {}", message);
    }

    /// Sends stringy data to the supplied Nym address
    pub fn send_str(&self, address: &str, message: &str) {}

    /// Sends bytes to the supplied Nym address
    pub fn send_bytes(&self, address: &str, message: Vec<u8>) {}

    fn new_nym_address() -> String {
        "the.nym@address".to_string()
    }

    fn some_minimal_config() -> Config {
        let mut keys_path = PathBuf::new();
        Config {
            keys: Some(keys_path),
        }
    }
}

pub struct Config {
    pub keys: Option<PathBuf>,
}
