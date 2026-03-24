[**@nymproject/sdk**](../globals.md) • **Docs**

***

[@nymproject/sdk](../globals.md) / Client

# Interface: Client

## Properties

### start()

> **start**: (`opts`?) => `Promise`\<`void`\>

Start the client.

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});

#### Parameters

• **opts?**: `any`

#### Returns

`Promise`\<`void`\>

#### Source

[mixnet/wasm/types.ts:33](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L33)

***

### stop()

> **stop**: () => `Promise`\<`void`\>

Stop the client.

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});
await client.stop();
```

#### Returns

`Promise`\<`void`\>

#### Source

[mixnet/wasm/types.ts:46](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L46)

***

### selfAddress()

> **selfAddress**: () => `Promise`\<`undefined` \| `string`\>

Get the client address

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});
const address = await client.selfAddress();
```

#### Returns

`Promise`\<`undefined` \| `string`\>

#### Source

[mixnet/wasm/types.ts:59](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L59)

***

### setTextMimeTypes()

> **setTextMimeTypes**: (`mimeTypes`) => `void`

Set the mime-types that should be used when using the [Client.send](Client.md#send) method.

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
clientId: 'my-client',
nymApiUrl: 'https://validator.nymtech.net/api',
});
await client.setTextMimeTypes(['text/plain', 'application/json']);
```

#### See

 - [MimeTypes](../enumerations/MimeTypes.md)
 - [Client.send](Client.md#send)
 - [Client.getTextMimeTypes](Client.md#gettextmimetypes)

#### Parameters

• **mimeTypes**: `string`[]

#### Returns

`void`

#### Source

[mixnet/wasm/types.ts:76](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L76)

***

### getTextMimeTypes()

> **getTextMimeTypes**: () => `Promise`\<`string`[]\>

Get the mime-types that are automatically converted to strings.

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
clientId: 'my-client',
nymApiUrl: 'https://validator.nymtech.net/api',
});
const mimeTypes = await client.getTextMimeTypes();
```

#### See

 - [MimeTypes](../enumerations/MimeTypes.md)
 - [Payload](Payload.md)
 - [Client.send](Client.md#send)
 - [Client.setTextMimeTypes](Client.md#settextmimetypes)

#### Returns

`Promise`\<`string`[]\>

#### Source

[mixnet/wasm/types.ts:93](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L93)

***

### send()

> **send**: (`args`) => `Promise`\<`void`\>

Send some data through the mixnet message.

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});
await client.send({
 payload: 'Hello world',
 recipient: // recipient address,
});
```

#### See

 - [MimeTypes](../enumerations/MimeTypes.md)
 - [Payload](Payload.md)

#### Parameters

• **args**

• **args.payload**: [`Payload`](Payload.md)

• **args.recipient**: `string`

• **args.replySurbs?**: `number`

#### Returns

`Promise`\<`void`\>

#### Source

[mixnet/wasm/types.ts:111](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L111)

***

### rawSend()

> **rawSend**: (`args`) => `Promise`\<`void`\>

Send a raw payload, without any mime-type conversion.

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});
const payload = new Uint8Array([1, 2, 3]);
await client.rawSend({
 payload,
 recipient: // recipient address,
});
```

#### See

 - [MimeTypes](../enumerations/MimeTypes.md)
 - [Payload](Payload.md)

#### Parameters

• **args**

• **args.payload**: `Uint8Array`

• **args.recipient**: `string`

• **args.replySurbs?**: `number`

#### Returns

`Promise`\<`void`\>

#### Source

[mixnet/wasm/types.ts:130](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L130)
