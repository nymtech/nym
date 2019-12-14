use nym_client::clients::directory;
use nym_client::clients::directory::presence::MixNodePresence;
use nym_client::clients::directory::requests::presence_mixnodes_post::PresenceMixNodesPoster;
use nym_client::clients::directory::DirectoryClient;
use std::thread;
use std::time::Duration;

pub struct Notifier {
    pub net_client: directory::Client,
    presence: MixNodePresence,
}

impl Notifier {
    pub fn new() -> Notifier {
        let config = directory::Config {
            base_url: "https://directory.nymtech.net/".to_string(),
        };
        let net_client = directory::Client::new(config);
        let presence = MixNodePresence {
            host: "halpin.org:6666".to_string(),
            pub_key: "superkey".to_string(),
            layer: 666,
            last_seen: 666,
        };
        Notifier {
            net_client,
            presence,
        }
    }

    pub fn notify(&self) {
        self.net_client
            .presence_mix_nodes_post
            .post(&self.presence)
            .unwrap();
    }

    pub fn run(&self) {
        loop {
            self.notify();
            thread::sleep(Duration::from_secs(5));
        }
    }
}
