use crate::validator::config::Config;
use crypto::identity::{
    DummyMixIdentityKeyPair, DummyMixIdentityPrivateKey, DummyMixIdentityPublicKey,
    MixnetIdentityKeyPair, MixnetIdentityPrivateKey, MixnetIdentityPublicKey,
};
use healthcheck::HealthChecker;
use log::debug;
use tokio::runtime::Runtime;

pub mod config;

// allow for a generic validator
pub struct Validator<IDPair, Priv, Pub>
where
    IDPair: MixnetIdentityKeyPair<Priv, Pub>,
    Priv: MixnetIdentityPrivateKey,
    Pub: MixnetIdentityPublicKey,
{
    #[allow(dead_code)]
    identity_keypair: IDPair,
    heath_check: HealthChecker<IDPair, Priv, Pub>,
}

// but for time being, since it's a dummy one, have it use dummy keys
impl Validator<DummyMixIdentityKeyPair, DummyMixIdentityPrivateKey, DummyMixIdentityPublicKey> {
    pub fn new(config: Config) -> Self {
        debug!("validator new");

        let dummy_keypair = DummyMixIdentityKeyPair::new();

        Validator {
            identity_keypair: dummy_keypair.clone(),
            heath_check: HealthChecker::new(config.health_check, dummy_keypair),
        }
    }

    pub fn start(self) {
        debug!("validator run");

        let mut rt = Runtime::new().unwrap();

        let health_check_future = self.heath_check.run();

        let health_check_res = rt.block_on(health_check_future);
        assert!(health_check_res.is_ok()); // panic if health checker failed
    }
}
