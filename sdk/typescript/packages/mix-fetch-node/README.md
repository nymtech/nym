# Nym MixFetch

This package is a drop-in replacement for `fetch` in NodeJS to send HTTP requests over the Nym Mixnet.

## Usage

```js
const { mixFetch } = require('@nymproject/mix-fetch-node-commonjs');

...

const response = await mixFetch('https://nymtech.net');
const html = await response.text();
```
