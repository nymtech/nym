use std::str::FromStr;

use cosmrs::crypto::secp256k1::{Signature, VerifyingKey};
use cosmrs::crypto::PublicKey;
use k256::ecdsa::signature::Verifier;

use crate::validator::signature::errors::Errors;

pub fn secp256k1_verify_with_public_key(
    public_key_as_bytes: &[u8],
    signature_as_hex: String,
    message: String,
) -> Result<(), k256::ecdsa::Error> {
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_as_bytes)?;
    let signature = Signature::from_str(&signature_as_hex)?;
    let message_as_bytes = message.into_bytes();
    verifying_key.verify(&message_as_bytes, &signature)
}

pub fn secp256k1_verify_with_public_key_json(
    public_key_as_json: String,
    signature_as_hex: String,
    message: String,
    account_id: String,
    account_prefix: &str,
) -> Result<(), Errors> {
    let public_key = PublicKey::from_json(&public_key_as_json)?;
    match public_key.account_id(account_prefix) {
        Ok(derived_account_id) => {
            if derived_account_id.to_string() != account_id {
                return Err(Errors::AccountIdError);
            }
            let verifying_key = VerifyingKey::from_sec1_bytes(&public_key.to_bytes())?;
            let signature = Signature::from_str(&signature_as_hex)?;
            let message_as_bytes = message.into_bytes();
            Ok(verifying_key.verify(&message_as_bytes, &signature)?)
        }
        Err(e) => Err(Errors::CosmrsError(e)),
    }
}

#[cfg(test)]
mod test_secp256k1 {
    use crate::validator::signature::helpers::{
        secp256k1_verify_with_public_key, secp256k1_verify_with_public_key_json,
    };
    use cosmrs::crypto::PublicKey;

    #[test]
    fn test_verify_with_json_public_key_with_valid_signature() {
        let json_public_key = r#"{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A4FdhUMasPmNhRZjtpKlmjNbq7EEUgPxfdI+E3vSajvc"}"#;
        let signature_as_hex = "E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28".to_string();
        let message = "test 1234".to_string();

        let public_key = PublicKey::from_json(json_public_key).unwrap();
        let public_key_bytes = public_key.to_bytes();

        let result = secp256k1_verify_with_public_key(&public_key_bytes, signature_as_hex, message);

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_with_json_public_key_with_invalid_signature() {
        let json_public_key = r#"{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A4FdhUMasPmNhRZjtpKlmjNbq7EEUgPxfdI+E3vSajvc"}"#;
        let signature_as_hex = "E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28".to_string();
        let message = "abcdef".to_string();

        let public_key = PublicKey::from_json(json_public_key).unwrap();
        let public_key_bytes = public_key.to_bytes();

        let result = secp256k1_verify_with_public_key(&public_key_bytes, signature_as_hex, message);

        assert!(result.is_err());
    }

    #[test]
    fn test_valid_json_public_key_succeeds() {
        let json_public_key = r#"{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A4FdhUMasPmNhRZjtpKlmjNbq7EEUgPxfdI+E3vSajvc"}"#.to_string();
        let signature_as_hex = "E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28".to_string();
        let message = "test 1234".to_string();
        let account_id = "n1lntkptzz8grf2w4yht4szxktzwsucgv4s7vv9g".to_string();
        let account_prefix = "n";

        let result = secp256k1_verify_with_public_key_json(
            json_public_key,
            signature_as_hex,
            message,
            account_id,
            account_prefix,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_json_public_key_fails_with_error() {
        let bad_json_public_key = r#"This is not JSON ☠️"#.to_string();
        let signature_as_hex = "E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28".to_string();
        let message = "abcdef".to_string();
        let account_id = "".to_string();
        let account_prefix = "n";

        let result = secp256k1_verify_with_public_key_json(
            bad_json_public_key,
            signature_as_hex,
            message,
            account_id,
            account_prefix,
        );

        assert!(result.is_err());
    }
}
