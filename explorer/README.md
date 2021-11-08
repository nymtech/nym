# Nym Network Explorer

The network explorer lets you explore the Nym network.

## Getting started

You will need:

- NodeJS (use `nvm install` to automatically install the correct version)
- `npm`

From the `explorer` directory of the `nym` monorepo, run:

```
npm install
npm run start
```

You can then open a browser to http://localhost:3000 and start development.

## Development

Documentation for developers [can be found here](./docs).

## Deployment

Build the UI with:

```
npm install
npm run build
```

The output will be in the `dist` directory. Serve this with `nginx` or `httpd`.

Make sure you have built the [explorer-api](./explorer-api) and are running it as a service proxied through 
`nginx` or `httpd` so that both the UI and API are running on the same host. 

## License

Please see https://github.com/nymtech/nym for license details for all Nym software.

## Contributing

If you would like to contribute to the Network Explorer send us a PR or
[raise an issue on GitHub](https://github.com/nymtech/nym/issues) and tag them with `network-explorer`.

## Development

Please see [development docs](./docs) here for more information on the structure and design of this app.
