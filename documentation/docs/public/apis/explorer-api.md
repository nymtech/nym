---
title: Explorer API (Deprecated)
description: Legacy Explorer API reference for the Nym Mixnet Explorer. Deprecated in favour of the Node Status API.
url: https://nym.com/docs/apis/explorer-api
---

# Explorer API

The Explorer API is deprecated. Use the [Node Status API](/apis/ns-api) instead, which provides the same data and more.

The Explorer API is the legacy backend for the [Mixnet Explorer](https://nym.com/explorer).

## Mainnet endpoints

- **OpenAPI spec:** [explorer.nymtech.net/api/v1/openapi.json](https://explorer.nymtech.net/api/v1/openapi.json)
- **Swagger UI:** [explorer.nymtech.net/api/swagger/index.html](https://explorer.nymtech.net/api/swagger/index.html)

<RedocStandalone
  specUrl="https://explorer.nymtech.net/api/v1/openapi.json"
  options={{
    nativeScrollbars: true,
    theme: {
      sidebar: {
        backgroundColor: '#273239',
        textColor: '#FCFDFE'
      }
    }
  }}
/>
