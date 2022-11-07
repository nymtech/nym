use nym_cli_commands::validator::signature::helpers::secp256k1_verify_with_public_key_json;

fn main() {
    println!("\nNym signature verification example\n\n");

    // the public key in JSON format (because Cosmos supports secp256k1 and ed25519 - NB: the helper only supports secp256k1)
    let public_key_as_json = r#"{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A4FdhUMasPmNhRZjtpKlmjNbq7EEUgPxfdI+E3vSajvc"}"#;

    // the Cosmos address prefix for Nyx is just 'n'
    let account_prefix = "n";

    // the account id is a hash of the public key combined with the prefix
    let account_id = "n1lntkptzz8grf2w4yht4szxktzwsucgv4s7vv9g".to_string();

    // the signature as a string of hex characters to represent the bytes in the signature
    let signature_as_hex = "E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28";

    // the original message as a string to verify
    let message = "test 1234".to_string();

    println!("public key: {}", &public_key_as_json);
    println!("account id: {}", &account_id);
    println!("signature:  {}", &signature_as_hex);
    println!("message:    {}", &message);

    println!();

    // this will pass, because the signature was signed for this message
    println!("\nVerify the correct message:");
    do_verify(
        account_id.clone(),
        account_prefix,
        public_key_as_json.to_string(),
        signature_as_hex.to_string(),
        message.clone(),
    );

    println!();

    // this will fail, because the account id doesn't match the public key
    println!("\nVerify the correct message with the wrong address:");
    do_verify(
        "n19s8wj0lhkvhr73vy746q3c2hfdzew80rxs6qmy".to_string(),
        account_prefix,
        public_key_as_json.to_string(),
        signature_as_hex.to_string(),
        message.clone(),
    );

    // this will fail, because the message was signed with another account private key
    println!("\nVerify the correct message with the wrong account and public key:");
    do_verify(
        "n19s8wj0lhkvhr73vy746q3c2hfdzew80rxs6qmy".to_string(),
        account_prefix,
        r#"{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A8l8JuPJjXPUWlBN0XaqRClrq9NMf2qaFQ5CJzidvAvK"}"#.to_string(),
        signature_as_hex.to_string(),
        message,
    );

    // this will fail, because the signature is for another message
    println!("\nVerify another message:");
    do_verify(
        account_id,
        account_prefix,
        public_key_as_json.to_string(),
        signature_as_hex.to_string(),
        "another message that will fail".to_string(),
    );
}

fn do_verify(
    account_id: String,
    prefix: &str,
    public_key_as_json: String,
    signature_as_hex: String,
    message: String,
) {
    match secp256k1_verify_with_public_key_json(
        public_key_as_json,
        signature_as_hex,
        message,
        account_id,
        prefix,
    ) {
        Ok(()) => println!("SUCCESS ✅ signature is valid"),
        Err(e) => println!("FAILURE ❌ signature is not valid: {}", e),
    }
}
