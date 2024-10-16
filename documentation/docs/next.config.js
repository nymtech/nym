// const path = require('path');
// const CopyPlugin = require('copy-webpack-plugin');

const withNextra = require("nextra")({
  theme: "nextra-theme-docs",
  themeConfig: "./theme.config.tsx",
});

const nextra = withNextra();
nextra.webpack = (config, options) => {
  // generate Nextra's webpack config
  const newConfig = withNextra().webpack(config, options);

  newConfig.module.rules.push({
    test: /\.txt$/i,
    use: "raw-loader",
  });

  // TODO: figure out how to properly bundle WASM and workers with Nextra
  // newConfig.plugins.push(
  //   new CopyPlugin({
  //     patterns: [
  //       {
  //         from: path.resolve(path.dirname(require.resolve('@nymproject/mix-fetch/package.json')), '*.wasm'),
  //         to: '[name][ext]',
  //         context: path.resolve(__dirname, 'out'),
  //       },
  //       {
  //         from: path.resolve(path.dirname(require.resolve('@nymproject/mix-fetch/package.json')), '*worker*.js'),
  //         to: '[name][ext]',
  //         context: path.resolve(__dirname, 'out'),
  //       },
  //     ],
  //   }),
  // );

  return newConfig;
};

const config = {
  ...nextra,
  // output: 'export', // static HTML files, has problems with Vercel
  // rewrites: undefined,
  async redirects() {
    return [
      // network docs
      {
        source: "/docs",
        destination: "/",
        permanent: true,
      },
      {
        source: "/docs/architecture/nym-vs-others.html",
        destination: "/network/architecture/nym-vs-others",
        permanent: true,
      },
      {
        source: "/docs/architecture/traffic-flow.html",
        destination: "/network/traffic",
        permanent: true,
      },
      {
        source: "/docs/architecture/addressing-system.html",
        destination: "/network/traffic/addressing-system",
        permanent: true,
      },
      {
        source: "/docs/binaries/pre-built-binaries.html",
        destination: "/developers/binaries#building-from-source",
        permanent: true,
      },
      {
        source: "/docs/binaries/init-and-config.html",
        destination: "/developers/binaries#building-from-source",
        permanent: true,
      },
      {
        source: "/docs/binaries/building-nym.html",
        destination: "/developers/binaries#building-from-source",
        permanent: true,
      },
      {
        source: "/docs/nodes/overview.html ",
        destination: "/network/architecture/mixnet/nodes",
        permanent: true,
      },
      {
        source: "/docs/wallet/desktop-wallet.html",
        destination:
          "https://github.com/nymtech/nym/tree/master/nym-wallet#installation-prerequisites---linux--mac",
        permanent: true,
      },
      {
        source: "/docs/wallet/cli-wallet.html",
        destination: "/developers/chain/cli-wallet",
        permanent: true,
      },
      {
        source: "/docs/explorers/mixnet-explorer.html",
        destination:
          "https://github.com/nymtech/nym/tree/master/explorer#nym-network-explorer",
        permanent: true,
      },
      {
        source: "/docs/nyx/interacting-with-chain.html",
        destination: "/developers/chain",
        permanent: true,
      },
      {
        source: "/docs/nyx/smart-contracts.html",
        destination: "/network/architecture/nyx/smart-contracts",
        permanent: true,
      },
      {
        source: "/docs/nyx/mixnet-contract.html",
        destination:
          "/network/architecture/nyx/smart-contracts/mixnet-contract",
        permanent: true,
      },
      {
        source: "/docs/nyx/vesting-contract.html",
        destination:
          "/network/architecture/nyx/smart-contracts/vesting-contract",
        permanent: true,
      },
      {
        source: "/docs/nyx/rpc-node.html",
        destination: "/developers/chain/rpc-node",
        permanent: true,
      },
      {
        source: "/docs/nyx/ledger-live.html",
        destination: "/developers/chain/ledger-live",
        permanent: true,
      },
      {
        source: "/docs/coconut.html",
        destination: "/network/cryptography/zk-nym",
        permanent: true,
      },
      {
        source: "/docs/nyx/bandwidth-credentials.html",
        destination: "/network/cryptography/zk-nym",
        permanent: true,
      },
      {
        source: "/docs/tools/nym-cli.html",
        destination: "/developers/tools/nym-cli",
        permanent: true,
      },
      {
        source: "/docs/coc.html",
        destination: "/network/coc",
        permanent: true,
      },
      {
        source: "/docs/licensing.html",
        destination: "/network/licensing",
        permanent: true,
      },
      // dev docs
      // operators docs TODO
    ];
  },
  images: {
    unoptimized: true,
  },
  transpilePackages: ["@nymproject/contract-clients"],
};

module.exports = config;
