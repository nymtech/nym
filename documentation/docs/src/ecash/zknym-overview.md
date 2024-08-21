# zkNym Generation and Usage: High Level Overview

```admonish info
Access to the Mixnet is - at the time of publication - free for everyone. However, soon™ it will be required for each connecting client to present a valid credential - a zkNym - to their ingress Gateway to access the Mixnet. This document outlines the payment flow and zkNym generation for **users of the NymVPN application only**.

Developers should not worry - individual developers will have access to something like a faucet for credentials, and larger application integrations will have their own 'under the hood' credential generation and distribution scheme for importing credentials into apps automatically. _More on this soon_.
```

Generation of zkNyms involves the following actors / pieces of infrastructure:
- NymVPN app instance: Users will generate a standard Bech32 address on the Nyx blockchain and use this mnemonic for login. The user is represented to the NymAPI Quorum with an ed25519 identity key.
- NymVPN OrderAPI: an API operating on behalf of users **not** paying in NYM tokens and swapping/depositing NYM with the NymAPI instances for payment verification.
- Nym Network: NymAPI LINK instances working together on Distributed Key Generation, refered to as the 'Nym API Quorum' + ingress Gateways LINK checking the validity of presented zkNym for Mixnet access.

Generation happens in 3 distinct stages:
- Subscription signup
- Deposit NYM tokens & issue zkNym
- Rerandomise and use zkNym for Nym Network access

The vast majority of this - from the perspective of the user - happens under the hood, but results in the creation and usage of an **unlinkable, rerandomisable anonymous proof of payment credential** with which users can use NymVPN without fear of doxxing themselves via linking app usage and payment information, just from interacting with a service payment provider in their NymVPN app in a way that they are used to doing.

## User Signup & Payment
- A Cosmos [Bech32 address](https://docs.cosmos.network/main/build/spec/addresses/bech32) is created for the user.
This is used to identify themselves when interacting with the NymVPN OrderAPI via signed authentication tokens. This is the only identity that the OrderAPI is able to see, and is not able to link this to generated zkNyms. This identity never leaves the user’s device and there is no email or any personal details needed for signup. _If a user is simply 'topping up' their subscription, the creation of the address is skipped as it already exists._
- The user can then interact (in-app) with various payment backends to pay for their subscription with non-NYM crypto, fiat options, or natively with NYM tokens.
- Payment options will trigger the OrderAPI. This will:
  - Create a swap for <PAYMENT_AMOUNT> <> NYM tokens.
  - Deposit these tokens with the NymAPI Quorum on the user's behalf.
- The user's device generates an ed25519 keypair: this is used to identify the device and authenticate it for using zkNyms as users can link several devices to the same account.
- The user's device then sends a request to each member of the Quorum requesting a zkNym.

## Deposit NYM & Issue zkNym
- Once NYM tokens have been deposited (( INTO A SMART CONTRACT? A MULTISIG? )) and a user requests a zkNym, each member of the Quroum create a partial blinded signature from their fragment of the key generated and split amongst them at the beginning of the Quroum. These blinded signatures are sent back to the requesting user.
- Once the user has enough signatures (the number of received signatures is > the required threshold) they can unblind and then aggregate them into a valid zkNym. This is fed into the local 'zkNym Generator'.

## Access Network
- The zkNym Generator is entirely offline and holds the zkNym locally on the user's device. Each time an application (in this case, NymVPN) requests an access credential, the Generator will rerandomise the zkNym and present this to the requesting party (the ingress Gateway for NymVPN / Mixnet-integrated app traffic).
- This zkNym is then presented to the Quorum by the ingress Gateway that collected it, which is used to calculate reward percentages given to Nym Network infrastructure operators.
