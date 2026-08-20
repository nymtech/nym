[**@nymproject/sdk**](../globals.md)

***

[@nymproject/sdk](../globals-1.md) / EventKinds

# Enumeration: EventKinds

Defined in: [mixnet/wasm/types.ts:202](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L202)

Enum representing various event kinds.

## Enumeration Members

### Loaded

> **Loaded**: `"Loaded"`

Defined in: [mixnet/wasm/types.ts:206](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L206)

The event emitted when the nodetester is ready to be used.

***

### Connected

> **Connected**: `"Connected"`

Defined in: [mixnet/wasm/types.ts:211](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L211)

The event emitted when connection to the gateway is established.

***

### StringMessageReceived

> **StringMessageReceived**: `"StringMessageReceived"`

Defined in: [mixnet/wasm/types.ts:216](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L216)

The event for when a message is received and interpreted as a string.

***

### BinaryMessageReceived

> **BinaryMessageReceived**: `"BinaryMessageReceived"`

Defined in: [mixnet/wasm/types.ts:221](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L221)

The event for when a binary message is received. BinaryMessage is a type of message that contains additional metadata, such as MIME type and some headers, along with the actual payload data.

***

### RawMessageReceived

> **RawMessageReceived**: `"RawMessageReceived"`

Defined in: [mixnet/wasm/types.ts:226](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L226)

The event for when a raw message is received. RawMessage represents the bytes that are received directly from the mixnet with no further parsing or interpretation done on them.
