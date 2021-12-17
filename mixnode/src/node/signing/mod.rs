use crypto::asymmetric::identity;

/// Signs text with the provided Ed25519 private key
pub fn sign_text(private_key: &identity::PrivateKey, text: &str) -> String {
    let signature_bytes = private_key.sign(text.as_ref()).to_bytes();
    let signature = bs58::encode(signature_bytes).into_string();
    signature
}
