# Nym MixFetch

This package is a drop-in replacement for `fetch` to send HTTP requests over the Nym Mixnet.

## Usage

Use `mixFetch` in your own project with:

```js
import { mixFetch } from '@nymproject/mix-fetch';

...

const response = await mixFetch('https://nymtech.net');
const html = await response.text();
```

