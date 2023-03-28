use nym_sphinx::addressing::clients::Recipient;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn validate_recipient(recipient: String) -> Result<(), JsError> {
    match Recipient::try_from_base58_string(recipient) {
        Ok(_) => Ok(()),
        Err(e) => Err(JsError::new(format!("{}", e).as_str())),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_recipient;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_recipient_validation_ok() {
        let res = validate_recipient("DyQmXnst5NGGjzUZxRC5Bjs5bd7CBF3xMpsSmbRiizr2.GH6YTBP2NXU3AVqd8WjiTMVyeMjunXMEsp7gVCMEJqpD@336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9".to_string());
        assert!(res.is_ok())
    }

    #[wasm_bindgen_test]
    fn test_recipient_validation_fails() {
        assert!(validate_recipient("  DyQmXnst5NGGjzUZxRC5Bjs5bd7CBF3xMpsSmbRiizr2.GH6YTBP2NXU3AVqd8WjiTMVyeMjunXMEsp7gVCMEJqpD@336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9".to_string()).is_err());
        assert!(validate_recipient(
            "DyQmXnst5NGGjzUZxRC5BjbRiizr2.GH6YTBP2NXU3AVqd8WD@336yuXAeGEgedRfqTJZQH1bHv1SjCZYarc9"
                .to_string()
        )
        .is_err());
        assert!(validate_recipient("🙀🙀🙀🙀".to_string()).is_err());
        assert!(validate_recipient("".to_string()).is_err());
        assert!(validate_recipient(" ".to_string()).is_err());
    }
}
