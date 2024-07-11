# Nym credential generation Usage Example

This is a simple project to show you how to use nym credential generation.

```ts
import { mixFetch } from '@nymproject/mix-fetch';

// HTTP GET
const response = await mixFetch('https://nymtech.net');
const html = await response.text();

// HTTP POST
const apiResponse = await mixFetch('https://api.example.com', { 
  method: 'POST', 
  body: JSON.stringify({ foo: 'bar' }), 
  headers: { [`Content-Type`]: 'application/json', Authorization: `Bearer ${AUTH_TOKEN}` }
});
```

## Running the example

```
npm install
npm run start
```

Open a browser at http://localhost:1234 and as the example loads, a connection will be made to the Nym Mixnet
and a text file and image will be downloaded and displayed in the browser.
