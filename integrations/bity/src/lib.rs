pub mod order;
pub mod sign;
pub mod verify;

#[cfg(test)]
mod tests {
    use crate::order::{Order, OrderSignature};
    use crate::{sign, verify};
    use cosmrs::AccountId;
    use std::str::FromStr;
    use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;

    fn get_order(prefix: &str) -> anyhow::Result<(OrderSignature, String)> {
        let mnemonic = "crush minute paddle tobacco message debate cabin peace bar jacket execute twenty winner view sure mask popular couch penalty fragile demise fresh pizza stove";
        let wallet = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic.parse()?);

        let accounts = wallet.try_derive_accounts()?;

        let message = "This is the order message from Bity";
        let signature = sign::sign_order(&wallet, &accounts[0], message.to_string())?;

        Ok((signature, message.to_string()))
    }

    #[test]
    fn integration_happy_path() -> anyhow::Result<()> {
        let (signature, message) = get_order("n")?;

        let account_id = AccountId::from_str("n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf").unwrap();
        assert_eq!(account_id.to_string(), signature.account_id.to_string());

        println!("Order signature:");
        println!("{}", ::serde_json::to_string_pretty(&signature)?);

        Ok(verify::verify_order(Order {
            account_id,
            signature,
            message,
        })?)
    }

    #[test]
    fn integration_fails_on_non_mainnet_address() -> anyhow::Result<()> {
        let (signature, message) = get_order("nymt")?;

        println!("Order signature:");
        println!("{}", ::serde_json::to_string_pretty(&signature)?);

        let res = verify::verify_order(Order {
            account_id: signature.clone().account_id,
            signature,
            message,
        });

        println!("Expecting error, got: {:?}", res);

        assert!(res.is_err());

        Ok(())
    }

    #[test]
    fn integration_fails_on_non_mainnet_address_variant2() -> anyhow::Result<()> {
        let (signature, message) = get_order("atom")?;

        println!("Order signature:");
        println!("{}", ::serde_json::to_string_pretty(&signature)?);

        let res = verify::verify_order(Order {
            account_id: signature.clone().account_id,
            signature,
            message,
        });

        println!("Expecting error, got: {:?}", res);

        assert!(res.is_err());

        Ok(())
    }

    #[test]
    fn integration_change_account_id() -> anyhow::Result<()> {
        let (signature, message) = get_order("n")?;

        let OrderSignature {
            signature_as_hex,
            public_key,
            account_id: _,
        } = signature;

        // use a different account id to the one that signed the order
        let account_id = AccountId::from_str("n1h5hgn94nsq4kh99rjj794hr5h5q6yfm2lr52es").unwrap();

        let res = verify::verify_order(Order {
            account_id: account_id.clone(),
            signature: OrderSignature {
                account_id,
                signature_as_hex,
                public_key,
            },
            message,
        });

        println!("Expecting error, got: {:?}", res);

        assert!(res.is_err());

        Ok(())
    }

    #[test]
    fn integration_change_signature() -> anyhow::Result<()> {
        let (signature, message) = get_order("n")?;

        let OrderSignature {
            signature_as_hex: _,
            public_key,
            account_id,
        } = signature;

        let res = verify::verify_order(Order {
            account_id: account_id.clone(),
            signature: OrderSignature {
                account_id,
                signature_as_hex: "this is not the signature you were looking for".to_string(),
                public_key,
            },
            message,
        });

        println!("Expecting error, got: {:?}", res);

        assert!(res.is_err());

        Ok(())
    }

    #[test]
    fn integration_with_json_happy_path() -> anyhow::Result<()> {
        let json_order = r#"{
  "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
  "message": "This is the order message from Bity",
  "signature": {
      "public_key": {
        "@type": "/cosmos.crypto.secp256k1.PubKey",
        "key": "A/zqdyeyPhCEXB9pyVLdNb5er+eds5ayboCdEEHK3Uom"
      },
      "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
      "signature_as_hex": "31C522B9B5C522A93CE14BE38E2D380CA166F69E952DF6F5D45B3B9CCDAAFE9115FBDF8539092986391C46885242E6E4CF806EEC1BB869A28D0E6D347C52121A"
  }
}"#;
        let order: Order = serde_json::from_str(json_order)?;

        Ok(verify::verify_order(order)?)
    }

    #[test]
    fn integration_with_json_bad_signature() -> anyhow::Result<()> {
        let json_order = r#"{
  "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
  "message": "A different message to the one signed",
  "signature": {
      "public_key": {
        "@type": "/cosmos.crypto.secp256k1.PubKey",
        "key": "A/zqdyeyPhCEXB9pyVLdNb5er+eds5ayboCdEEHK3Uom"
      },
      "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
      "signature_as_hex": "31C522B9B5C522A93CE14BE38E2D380CA166F69E952DF6F5D45B3B9CCDAAFE9115FBDF8539092986391C46885242E6E4CF806EEC1BB869A28D0E6D347C52121A"
  }
}"#;
        let order: Order = serde_json::from_str(json_order)?;
        let res = verify::verify_order(order);
        println!("Expecting error, got: {:?}", res);
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn integration_with_json_bad_account_id() -> anyhow::Result<()> {
        let json_order = r#"{
  "account_id": "n1h5hgn94nsq4kh99rjj794hr5h5q6yfm2lr52es",
  "message": "This is the order message from Bity",
  "signature": {
      "public_key": {
        "@type": "/cosmos.crypto.secp256k1.PubKey",
        "key": "A/zqdyeyPhCEXB9pyVLdNb5er+eds5ayboCdEEHK3Uom"
      },
      "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
      "signature_as_hex": "31C522B9B5C522A93CE14BE38E2D380CA166F69E952DF6F5D45B3B9CCDAAFE9115FBDF8539092986391C46885242E6E4CF806EEC1BB869A28D0E6D347C52121A"
  }
}"#;
        let order: Order = serde_json::from_str(json_order)?;
        let res = verify::verify_order(order);
        println!("Expecting error, got: {:?}", res);
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn integration_with_json_bad_account_id_variation_2() -> anyhow::Result<()> {
        let json_order = r#"{
  "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
  "message": "This is the order message from Bity",
  "signature": {
      "public_key": {
        "@type": "/cosmos.crypto.secp256k1.PubKey",
        "key": "A/zqdyeyPhCEXB9pyVLdNb5er+eds5ayboCdEEHK3Uom"
      },
      "account_id": "n1h5hgn94nsq4kh99rjj794hr5h5q6yfm2lr52es",
      "signature_as_hex": "31C522B9B5C522A93CE14BE38E2D380CA166F69E952DF6F5D45B3B9CCDAAFE9115FBDF8539092986391C46885242E6E4CF806EEC1BB869A28D0E6D347C52121A"
  }
}"#;
        let order: Order = serde_json::from_str(json_order)?;
        let res = verify::verify_order(order);
        println!("Expecting error, got: {:?}", res);
        assert!(res.is_err());
        Ok(())
    }
}
