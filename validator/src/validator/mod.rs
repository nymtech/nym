use crate::validator::config::Config;
use log::debug;

pub mod config;
mod health_check;

pub struct Validator {}

impl Validator {
    pub fn new(config: &Config) -> Self {
        debug!("validator new");

        Validator {}
    }

    pub fn start(self) {
        debug!("validator run");
    }
}
