use crate::crypto::{EphemeraKeypair, Keypair};
use clap::Parser;

use crate::utilities::crypto::EphemeraPublicKey;

#[derive(Debug, Clone, Parser)]
pub struct GenerateKeypairCmd;

impl GenerateKeypairCmd {
    pub fn execute() {
        let keypair = Keypair::generate(None);
        println!("Keypair: {:>5}", keypair.to_base58());
        println!("Public key: {:>5}", keypair.public_key().to_base58());
    }
}
