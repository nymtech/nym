use crate::network::tendermint_abci;
use crypto::identity::{
    DummyMixIdentityKeyPair, DummyMixIdentityPrivateKey, DummyMixIdentityPublicKey,
    MixnetIdentityKeyPair, MixnetIdentityPrivateKey, MixnetIdentityPublicKey,
};
use healthcheck::HealthChecker;
use log::*;
use tokio::runtime::Runtime;

use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(rename(deserialize = "healthcheck"))]
    pub health_check: healthcheck::config::HealthCheck,
}

// allow for a generic validator
pub struct Validator<IDPair, Priv, Pub>
where
    IDPair: MixnetIdentityKeyPair<Priv, Pub>,
    Priv: MixnetIdentityPrivateKey,
    Pub: MixnetIdentityPublicKey,
{
    pub count: u64,
    heath_check: HealthChecker<IDPair, Priv, Pub>,
    #[allow(dead_code)]
    identity_keypair: IDPair,
}

// but for time being, since it's a dummy one, have it use dummy keys
impl Validator<DummyMixIdentityKeyPair, DummyMixIdentityPrivateKey, DummyMixIdentityPublicKey> {
    pub fn new(config: Config) -> Self {
        debug!("validator new");

        let dummy_keypair = DummyMixIdentityKeyPair::new();

        Validator {
            count: 0,
            heath_check: HealthChecker::new(config.health_check, dummy_keypair.clone()),
            identity_keypair: dummy_keypair,
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
