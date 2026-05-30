[**@nymproject/mix-fetch**](../globals.md) • **Docs**

***

[@nymproject/mix-fetch](../globals.md) / ResponseBodyConfigMap

# Interface: ResponseBodyConfigMap

## Properties

### uint8array?

> `optional` **uint8array**: (`string` \| `RegExp`)[]

Set the response `Content-Type`s to decode as uint8array.

#### Source

[types.ts:49](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/types.ts#L49)

***

### json?

> `optional` **json**: (`string` \| `RegExp`)[]

Set the response `Content-Type`s to decode with the `json()` response body method.

#### Source

[types.ts:54](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/types.ts#L54)

***

### text?

> `optional` **text**: (`string` \| `RegExp`)[]

Set the response `Content-Type`s to decode with the `text()` response body method.

#### Source

[types.ts:59](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/types.ts#L59)

***

### formData?

> `optional` **formData**: (`string` \| `RegExp`)[]

Set the response `Content-Type`s to decode with the `formData()` response body method.

#### Source

[types.ts:64](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/types.ts#L64)

***

### blob?

> `optional` **blob**: (`string` \| `RegExp`)[]

Set the response `Content-Type`s to decode with the `blob()` response body method.

#### Source

[types.ts:69](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/types.ts#L69)

***

### fallback?

> `optional` **fallback**: [`ResponseBodyMethod`](../type-aliases/ResponseBodyMethod.md)

Set this to the default fallback method. Set to `undefined` if you want to ignore unknown types.

#### Source

[types.ts:74](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/types.ts#L74)
