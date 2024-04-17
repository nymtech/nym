# Bonding Nym Node

```admonish caution
If you unbond your Nym Node that means you are leaving the mixnet and you will lose all your delegations (permanently). You can join again with the same identity key, however, you will start with **no delegations**.
```

Nym Mixnet operators are rewarded for their work every epoch (60 minutes). To prevent centralisation, [Nym API](nym-api.md) is ran by distributed validators on Nyx blockchain.

You are asked to `sign` a transaction and bpnd your node to Nyx blockchain so that the Mixnet smart contract is able to map your nym address to your node. This allows us to create a nonce for each account and defend against replay attacks.

**Before you bond your `nym-node` make sure you went through all the previous steps**

1. [Build](../binaries/building-nym.md) or [download](../binaries/pre-build-binaries.md) `nym-node` binary
2. [Configure VPS](vps-setup.md) correctly
3. [Prepare Nym wallet](wallet-preparation.md)
4. [Setup & Run](setup.md) the node
5. (Optional but reccomended) [Configure](configuration.md) the node (WSS, reversed proxy, automation)

```admonish warning
Do not bond your node to the API if the previous steps weren't finished. Bad connectivity, closed ports, or other poor setup will result in your node getting blacklisted.
```

## Bond via the Desktop wallet (recommended)

You can bond your `nym-node` via the Desktop wallet.

1. Open your wallet, and head to the `Bond` page, then select the node type `Mixnode` and input your node details. Press `Next`.
  - To find out your `nym-node` details, run `./nym-node bonding-information`
  - To get a correct host address, run `echo "$(curl -4 https://ifconfig.me)"`


2. Enter the `Amount`, `Operating cost` and `Profit margin` and press `Next`.

3. You will be asked to run a `sign` command with your `nym-node` - copy and paste the long signature as the value of `--contract-msg` and run it.

```
./nym-node sign --contract-msg <PAYLOAD_GENERATED_BY_THE_WALLET>
```
<!-- NEED RE-RUN
It will look something like this:

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-mixnode init --id my-node --host $(curl -4 https://ifconfig.me) -->
<!-- cmdrun ../../../../target/release/nym-mixnode sign --id my-node --contract-msg 22Z9wt4PyiBCbMiErxj5bBa4VCCFsjNawZ1KnLyMeV9pMUQGyksRVANbXHjWndMUaXNRnAuEVJW6UCxpRJwZe788hDt4sicsrv7iAXRajEq19cWPVybbUqgeo76wbXbCbRdg1FvVKgYZGZZp8D72p5zWhKSBRD44qgCrqzfV1SkiFEhsvcLUvZATdLRocAUL75KmWivyRiQjCE1XYEWyRH9yvRYn4TymWwrKVDtEB63zhHjATN4QEi2E5qSrSbBcmmqatXsKakbgSbQoLsYygcHx7tkwbQ2HDYzeiKP1t16Rhcjn6Ftc2FuXUNnTcibk2LQ1hiqu3FAq31bHUbzn2wiaPfm4RgqTwGM4eqnjBofwR3251wQSxbYwKUYwGsrkweRcoPuEaovApR9R19oJ7GVG5BrKmFwZWX3XFVuECe8vt1x9MY7DbQ3xhAapsHhThUmzN6JPPU4qbQ3PdMt3YVWy6oRhap97ma2dPMBaidebfgLJizpRU3Yu7mtb6E8vgi5Xnehrgtd35gitoJqJUY5sB1p6TDPd6vk3MVU1zqusrke7Lvrud4xKfCLqp672Bj9eGb2wPwow643CpHuMkhigfSWsv9jDq13d75EGTEiprC2UmWTzCJWHrDH7ka68DZJ5XXAW67DBewu7KUm1jrJkNs55vS83SWwm5RjzQLVhscdtCH1Bamec6uZoFBNVzjs21o7ax2WHDghJpGMxFi6dmdMCZpqn618t4 -->
```
~~~
-->
4. Copy the resulting signature string and paste it into the wallet nodal, press `Next` and confirm the transaction:

```sh
# This is just an example, copy the one from your process
>>> The base58-encoded signature is:
2bbDJSmSo9r9qdamTNygY297nQTVRyQaxXURuomVcRd7EvG9oEC8uW8fvZZYnDeeC9iWyG9mAbX2K8rWEAxZBro1
```

![Paste Signature](../images/wallet-screenshots/wallet-sign.png)
*This image is just an example, copy-paste your own base58-encoded signature*

5. Your node will now be bonded and ready to recieve traffic, latest at the beginning of the next epoch (at most 1 hour)


If everything worked, you'll see your node running on the either the [Sandbox testnet network explorer](https://sandbox-explorer.nymtech.net) or the [mainnet network explorer](https://explorer.nymtech.net), depending on which network you're running.


### Bond via the CLI (power users)
If you want to bond your Mix Node via the CLI, then check out the [relevant section in the Nym CLI](https://nymtech.net/docs/tools/nym-cli.html#bond-a-mix-node) docs.
