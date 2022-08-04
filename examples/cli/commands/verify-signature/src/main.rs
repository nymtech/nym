use nym_cli_commands::validator::signature::helpers::secp256k1_verify_with_public_key_json;

fn main() {
    println!("\nNym signature verification example\n\n");

    // the public key in JSON format (because Cosmos supports secp256k1 and ed25519 - NB: the helper only supports secp256k1)
    let public_key_as_json = r#"{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A4FdhUMasPmNhRZjtpKlmjNbq7EEUgPxfdI+E3vSajvc"}"#;

    // the signature as a string of hex characters to represent the bytes in the signature
    let signature_as_hex = "E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28";

    // the original message as a string to verify
    let message = "test 1234".to_string();

    println!("public key: {}", &public_key_as_json);
    println!("signature:  {}", &signature_as_hex);
    println!("message:    {}", &message);

    println!();

    // this will pass, because the signature was signed for this message
    println!("\nVerify the correct message:\n");
    do_verify(
        public_key_as_json.to_string(),
        signature_as_hex.to_string(),
        message,
    );

    // this will fail, because the signature is for another message
    println!("\n\nVerify another message:\n");
    do_verify(
        public_key_as_json.to_string(),
        signature_as_hex.to_string(),
        "another message that will fail".to_string(),
    );
}

fn do_verify(public_key_as_json: String, signature_as_hex: String, message: String) {
    match secp256k1_verify_with_public_key_json(public_key_as_json, signature_as_hex, message) {
        Ok(()) => println!("SUCCESS ✅ signature is valid"),
        Err(e) => println!("FAILURE ❌ signature is not valid: {}", e),
    }
}
