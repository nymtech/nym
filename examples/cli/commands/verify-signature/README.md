# Example code to verify a signature

This is an example app that shows how to verify a signature signed by an account key.

Inputs to the app are:

- **signature** - bytes represented as a hex string
- **public key** - in JSON format (you can query any Cosmos chain for account's public key as JSON, however it will need to have sent a signed transaction to the chain for this to be present)
- **message** - the string message to verify

## Running locally

Run the example by changning to this directory and running:

```
cargo run
```

And you should see the output:

```
Nym signature verification example


public key: {"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A4FdhUMasPmNhRZjtpKlmjNbq7EEUgPxfdI+E3vSajvc"}
signature:  E3AA5AC0DA1B7DEBB7808000F719D8ACB9A0BE10AFA2756A788516268EB246A1257EC1097C5E364EF916145B01641DEDFE955994CB340BDAFA99A65BCA3F6F28
message:    test 1234


Verify the correct message:

SUCCESS ✅ signature is valid


Verify another message:

FAILURE ❌ signature is not valid: signature error
```