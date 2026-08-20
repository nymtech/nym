[**@nymproject/sdk**](../globals.md)

***

[@nymproject/sdk](../globals-1.md) / Client

# Interface: Client

Defined in: [mixnet/wasm/types.ts:19](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L19)

## Properties

### start

> **start**: (`opts?`) => `Promise`\<`void`\>

Defined in: [mixnet/wasm/types.ts:33](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L33)

Start the client.

#### Parameters

##### opts?

`ClientOpts`

#### Returns

`Promise`\<`void`\>

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});

***

### stop

> **stop**: () => `Promise`\<`void`\>

Defined in: [mixnet/wasm/types.ts:46](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L46)

Stop the client.

#### Returns

`Promise`\<`void`\>

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});
await client.stop();
```

***

### selfAddress

> **selfAddress**: () => `Promise`\<`string` \| `undefined`\>

Defined in: [mixnet/wasm/types.ts:59](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L59)

Get the client address

#### Returns

`Promise`\<`string` \| `undefined`\>

#### Example

```typescript
const client = await createNymMixnetClient();
await client.start({
 clientId: 'my-client',
 nymApiUrl: 'https://validator.nymtech.net/api',
});
const address = await client.selfAddress();
```

***

### setTextMimeTypes

> **setTextMimeTypes**: (`mimeTypes`) => `void`

Defined in: [mixnet/wasm/types.ts:76](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L76)

Set the mime-types that should be used when using the [Client.send](#send) method.

#### Parameters

##### mimeTypes

`string`[]

#### Returns

`void`

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
 - [Client.send](#send)
 - [Client.getTextMimeTypes](#gettextmimetypes)

***

### getTextMimeTypes

> **getTextMimeTypes**: () => `Promise`\<`string`[]\>

Defined in: [mixnet/wasm/types.ts:93](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L93)

Get the mime-types that are automatically converted to strings.

#### Returns

`Promise`\<`string`[]\>

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
 - [Client.send](#send)
 - [Client.setTextMimeTypes](#settextmimetypes)

***

### send

> **send**: (`args`) => `Promise`\<`void`\>

Defined in: [mixnet/wasm/types.ts:111](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L111)

Send some data through the mixnet message.

#### Parameters

##### args

###### payload

[`Payload`](Payload.md)

###### recipient

`string`

###### replySurbs?

`number`

#### Returns

`Promise`\<`void`\>

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

***

### rawSend

> **rawSend**: (`args`) => `Promise`\<`void`\>

Defined in: [mixnet/wasm/types.ts:130](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L130)

Send a raw payload, without any mime-type conversion.

#### Parameters

##### args

###### payload

`Uint8Array`

###### recipient

`string`

###### replySurbs?

`number`

#### Returns

`Promise`\<`void`\>

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
