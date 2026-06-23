[**@nymproject/mix-dns**](../globals.md) • **Docs**

***

[@nymproject/mix-dns](../globals.md) / mixDNS

# Function: mixDNS()

> **mixDNS**(`hostname`): `Promise`\<`string`\>

Resolve a hostname through the mixnet. Returns the IP as a string
(e.g. `"93.184.216.34"`).

The tunnel must already be set up via `setupMixTunnel()`.

## Parameters

• **hostname**: `string`

## Returns

`Promise`\<`string`\>

## Source

[mix-dns/src/index.ts:23](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-dns/src/index.ts#L23)
