# NYM API mock

This package provides a mock server that allows you to modify parts of the Nym API by:

- modifying live responses from a running Nym API
- providing a static response

## How to run it?

Adjust the [.env](./.env) to use the Nym API you want to proxy. The defaults are for mainnet.

From this directory run:

```
yarn
yarn start
```

When you modify files in `src` and `mocks` the process will be restarted automatically.

## How can I find out what methods I can override?

Look in the swagger docs for the Nym API, e.g. https://validator.nymtech.net/api/swagger/index.html.

Then write a handler to override it, for example, to return custom gateways:

```ts
app.get('/api/v1/gateways', (req, res) => {
  const customGateways = JSON.parse(fs.readFileSync('./mocks/custom-gateway.json').toString());

  // modify custom gateway
  customGateways[0].gateway.sphinx_key += '-ccc';
  customGateways[0].gateway.identity_key += '-ddd';

  res.json(customGateways);
});
```

## How to get seed data?

You can get seed data from a running Nym API with:

```
curl https://validator.nymtech.net/api/v1/mixnodes | jq . > mocks/mixnodes.json
```

If you don't have `jq` installed, you can just do `curl https://validator.nymtech.net/api/v1/mixnodes > mocks/mixnodes.json` to store the unformatted response.

## HTTPS
 
If you need HTTPS then install `caddy` (see https://caddyserver.com/ or `brew install caddy` on MacOS) and run:

```
yarn
yarn start:https
```

The API will available on `https://localhost:8001` e.g. https://localhost:8001/api/v1/mixnodes with a self-signed certificate.

Modify the [Caddyfile](Caddyfile) to set the domain for the certificate (NB: you will need to set up the DNS and control the domain records).

If you need the local certificate to be valid, look at https://github.com/FiloSottile/mkcert and https://dev.to/josuebustos/https-localhost-for-node-js-1p1k.

