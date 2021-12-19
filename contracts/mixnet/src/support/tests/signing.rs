// use ed25519_dalek::{Keypair, PublicKey, Signature, Signer};

// use rand::rngs::OsRng;

// /// A test helper which signs a piece of text with a supplied private key
// /// This is test-only code, not meant for use inside the smart contract.
// /// It allows us to sign things so we can test that verification works inside
// /// the smart contract.
// pub(crate) fn sign(text: &str) -> (Signature, PublicKey) {
//     let mut csprng = OsRng {};
//     let keypair: Keypair = Keypair::generate(&mut csprng);
//     let signature: Signature = keypair.sign(text.as_bytes());
//     (signature, keypair.public)
// }
