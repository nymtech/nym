# Buy NYM with Bity

This crate allows Bity to verify orders for purchasing NYM tokens. The same crate is used by the wallet to sign orders for purchases.

## Signing

The Nym Wallet user will sign an order message provided by Bity to create a signed order with the following fields:

```
account_id: n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf
message: "This is the order message from Bity"
order signature: 
{
  "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
  "public_key": {
    "@type": "/cosmos.crypto.secp256k1.PubKey",
    "key": "A/zqdyeyPhCEXB9pyVLdNb5er+eds5ayboCdEEHK3Uom"
  },
  "signature_as_hex": "31C522B9B5C522A93CE14BE38E2D380CA166F69E952DF6F5D45B3B9CCDAAFE9115FBDF8539092986391C46885242E6E4CF806EEC1BB869A28D0E6D347C52121A"
}
```

The `signature` field of the order contains a JSON representation of:

- the Cosmos address of the signer (`account_id`)
- the Cosmos public key
- a hex string digest of the Bity order message signed by the user

Note: the `signature_as_hex` is not in recoverable form (e.g. allows recovering the public key from the signature in `secp256k1`). This is why the public key is supplied along with the account id, as the prefix cannot be recovered.

## Verification

Verification has been wrapped up into taking a single struct that can be parsed from JSON:

```
{
  "account_id": "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
  "message": "This is the order message from Bity",
  "signature": << ORDER SIGNATURE JSON GOES HERE >>
}
```

The following will be checked:

- the `account_id` supplied matches:
  - the account id derived from the public key
  - the account id field in the order signature JSON
- the account id is for Nym mainnet
- the signature is for the message
- all data structures parse correctly
  - nested structs
  - account ids
  - Cosmos public keys