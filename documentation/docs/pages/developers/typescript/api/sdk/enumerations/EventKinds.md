[**@nymproject/sdk**](../README.md) • **Docs**

***

[@nymproject/sdk](../globals.md) / EventKinds

# Enumeration: EventKinds

Enum representing various event kinds.

## Enumeration Members

### Loaded

> **Loaded**: `"Loaded"`

The event emitted when the nodetester is ready to be used.

#### Source

[mixnet/wasm/types.ts:206](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L206)

***

### Connected

> **Connected**: `"Connected"`

The event emitted when connection to the gateway is established.

#### Source

[mixnet/wasm/types.ts:211](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L211)

***

### StringMessageReceived

> **StringMessageReceived**: `"StringMessageReceived"`

The event for when a message is received and interpreted as a string.

#### Source

[mixnet/wasm/types.ts:216](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L216)

***

### BinaryMessageReceived

> **BinaryMessageReceived**: `"BinaryMessageReceived"`

The event for when a binary message is received. BinaryMessage is a type of message that contains additional metadata, such as MIME type and some headers, along with the actual payload data.

#### Source

[mixnet/wasm/types.ts:221](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L221)

***

### RawMessageReceived

> **RawMessageReceived**: `"RawMessageReceived"`

The event for when a raw message is received. RawMessage represents the bytes that are received directly from the mixnet with no further parsing or interpretation done on them.

#### Source

[mixnet/wasm/types.ts:226](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L226)
