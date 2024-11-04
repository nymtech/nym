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
  basePath: "/docs",
  async redirects() {
    return [
      // network docs
      // {
      //   source: "/",
      //   destination: "/docs",
      //   basePath: false,
      //   permanent: true,
      // },
{
        source: "/operators",
        destination: "docs/operators/introduction",
        permanent: true,
        basePath : false,
      },

      {
        source: "/developers",
        destination: "/docs/developers/index",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/architecture/nym-vs-others.html",
        destination: "/docs/network/architecture/nym-vs-others",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/architecture/traffic-flow.html",
        destination: "/docs/network/traffic", // testing difference
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/architecture/addressing-system.html",
        destination: "/docs/network/traffic/addressing-system",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/binaries/pre-built-binaries.html",
        destination: "/docs/developers/binaries#building-from-source",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/binaries/init-and-config.html",
        destination: "/docs/developers/binaries#building-from-source",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/binaries/building-nym.html",
        destination: "/docs/developers/binaries#building-from-source",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nodes/overview.html ",
        destination: "/docs/network/architecture/mixnet/nodes",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/wallet/desktop-wallet.html",
        destination:
          "https://github.com/nymtech/nym/tree/master/nym-wallet#installation-prerequisites---linux--mac",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/wallet/cli-wallet.html",
        destination: "/docs/developers/chain/cli-wallet",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/explorers/mixnet-explorer.html",
        destination:
          "https://github.com/nymtech/nym/tree/master/explorer#nym-network-explorer",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/interacting-with-chain.html",
        destination: "/docs/developers/chain",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/smart-contracts.html",
        destination: "/docs/network/architecture/nyx/smart-contracts",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/mixnet-contract.html",
        destination:
          "/docs/network/architecture/nyx/smart-contracts/mixnet-contract",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/vesting-contract.html",
        destination:
          "/docs/network/architecture/nyx/smart-contracts/vesting-contract",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/rpc-node.html",
        destination: "/docs/developers/chain/rpc-node",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/ledger-live.html",
        destination: "/docs/developers/chain/ledger-live",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/coconut.html",
        destination: "/docs/network/cryptography/zk-nym",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/nyx/bandwidth-credentials.html",
        destination: "/docs/network/cryptography/zk-nym",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/tools/nym-cli.html",
        destination: "/docs/developers/tools/nym-cli",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/coc.html",
        destination: "/docs/network/coc",
        permanent: true,
        basePath : false,
      },
      {
        source: "/docs/licensing.html",
        destination: "/docs/network/licensing",
        permanent: true,
        basePath : false,
      },
      // dev docs
      {
        source: "/developers/clients-overview.html",
        destination: "/docs/developers/clients",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/rust.html",
        destination: "/docs/developers/rust",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/message-types.html",
        destination: "/docs/developers/rust/mixnet/message-types",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/message-helpers.html",
        destination: "/docs/developers/rust/mixnet/message-helpers",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/troubleshooting.html",
        destination: "/docs/developers/rust/mixnet/troubleshooting",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples.html",
        destination: "/docs/developers/rust/mixnet/examples",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/simple.html",
        destination: "/docs/developers/rust/mixnet/examples/simple",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/keys.html",
        destination: "/docs/developers/sdk/rust/examples/keys.html",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/storage.html",
        destination:
          "/docs/developers/rust/mixnet/examples/builders/builder-with-storage",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/surbs.html",
        destination: "/docs/developers/rust/mixnet/examples/surbs",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/custom-network.html",
        destination: "/docs/developers/rust/mixnet/examples/custom-topology",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/socks.html",
        destination: "/docs/developers/rust/mixnet/examples/socks",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/split-send.html",
        destination: "/docs/developers/rust/mixnet/examples/split-send",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/credential.html",
        destination: "/docs/developers/rust/mixnet",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/rust/examples/cargo.html",
        destination: "/docs/developers/rust/importing",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/sdk/typescript.html",
        destination: "/docs/developers/typescript",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/binaries/pre-built-binaries.html",
        destination: "/docs/developers/binaries#pre-built-binaries",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/binaries/building-nym.html",
        destination: "/docs/developers/binaries",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/websocket-client.html",
        destination: "/docs/developers/clients/websocket",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/websocket/setup.html",
        destination: "/docs/developers/clients/websocket/setup",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/websocket/config.html",
        destination: "/docs/developers/clients/websocket/config",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/websocket/usage.html",
        destination: "/docs/developers/clients/websocket/usage",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/websocket/examples.html",
        destination: "/docs/developers/clients/websocket/examples",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/socks5-client.html",
        destination: "/docs/developers/clients/socks5",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/socks5/setup.html",
        destination: "/docs/developers/clients/socks5#client-setup",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/socks5/usage.html",
        destination: "/docs/developers/clients/socks5#using-your-socks5-client",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/clients/webassembly-client.html",
        destination: "/docs/developers/clients/webassembly-client",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/tutorials/coming-soon.html",
        destination: "/docs/developers/rust#",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/integrations/integration-options.html",
        destination: "/docs/developers/integrations",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/faq/integrations-faq.html",
        destination: "/docs/developers/integrations",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/coc.html",
        destination: "/docs/developers/coc",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/licensing.html",
        destination: "/docs/developers/licensing",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/nymvpn/intro.html",
        destination: "/docs/developers/archive/nymvpn",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/nymvpn/cli.html",
        destination: "/docs/developers/nymvpn/cli",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/archive/nym-connect.html",
        destination: "/docs/developers/archive/nym-connect",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/examples/custom-services.html",
        destination: "/docs/developers/rust/mixnet/other-examples#services",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/examples/browser-only.html",
        destination: "/docs/developers/rust/mixnet/other-examples#browser-only",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/examples/using-nrs.html",
        destination: "/docs/developers/rust/mixnet/other-examples",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/examples/monorepo-examples.html",
        destination: "/docs/developers/rust/mixnet/other-examples",
        permanent: true,
        basePath : false,
      },
      {
        source: "/developers/integrations",
        destination: "/docs/developers/integrations/payment-integration.html",
        permanent: true,
        basePath : false,
      },

      // operators:
      // specific urls that have changed
      {
        source: "/operators/nodes/wallet-preparation.html",
        destination: "/docs/operators/nodes/preliminary-steps/wallet-preparation",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/vps-setup.html",
        destination: "/docs/operators/nodes/preliminary-steps/vps-setup",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/proxy-configuration.html",
        destination:
          "/docs/operators/nodes/nym-node/configuration/proxy-configuration",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/bonding.html",
        destination: "/docs/operators/nodes/nym-node/bonding",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/nym-api.html",
        destination: "/docs/operators/nodes/validator-setup/nym-api",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/nyx-configuration.html",
        destination: "/docs/operators/nodes/validator-setup/nyx-configuration",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/manual-upgrade.html",
        destination: "/docs/operators/nodes/maintenance/manual-upgrade",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/nodes/nymvisor-upgrade.html",
        destination: "/docs/operators/nodes/maintenance/nymvisor-upgrade",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/testing/performance.html",
        destination: "/docs/operators/nodes/performance-and-testing",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/testing/gateway-probe.html",
        destination: "/docs/operators/nodes/performance-and-testing/gateway-probe",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/testing/node-api-check.html",
        destination: "/docs/operators/nodes/performance-and-testing/node-api-check",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/testing/prometheus-grafana.html",
        destination:
          "/docs/operators/nodes/performance-and-testing/prometheus-grafana",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/testing/explorenym-scripts.html",
        destination:
          "/docs/operators/nodes/performance-and-testing/prometheus-grafana/explorenym-scripts",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/community-counsel.html",
        destination: "/docs/operators/community-counsel",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/exit-gateway.html",
        destination: "/docs/operators/community-counsel/exit-gateway",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/isp-list.html",
        destination: "/docs/operators/community-counsel/isp-list",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/jurisdictions.html",
        destination: "/docs/operators/community-counsel/jurisdictions",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/swiss.html",
        destination: "/docs/operators/community-counsel/jurisdictions/swiss",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/united-states.html",
        destination: "/docs/operators/community-counsel/jurisdictions/united-states",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/landing-pages.html",
        destination: "/docs/operators/community-counsel/landing-pages",
        permanent: true,
        basePath : false,
      },
      {
        source: "/operators/legal/add-content.html",
        destination: "/docs/operators/community-counsel/add-content",
        permanent: true,
        basePath : false,
      },

      // Change the basePath to /docs
      {
        source: "/",
        destination: "/docs",
        basePath: false,
        permanent: true,
      },
      // TODO these need to go in the config of the existing deployed ts sdk docs to redirect from there
      //      these assume source basePath = sdk.nymtech.net
      // {
      //   source: "/intro",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript",
      //   permanent: true,
      // },
      // {
      //   source: "/overview",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/overview",
      //   permanent: true,
      // },
      // {
      //   source: "/integrations",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/integrations",
      //   permanent: true,
      // },
      // {
      //   source: "/installation",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/installation",
      //   permanent: true,
      // },
      // {
      //   source: "/start",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/start",
      //   permanent: true,
      // },
      // {
      //   source: "/examples/mix-fetch",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>developers/typescript/examples/mix-fetch",
      //   permanent: true,
      // },
      // {
      //   source: "/examples/mixnet",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>//developers/typescript/examples/mixnet",
      //   permanent: true,
      // },
      // {
      //   source: "/examples/nym-smart-contracts",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/examples/nym-smart-contracts",
      //   permanent: true,
      // },
      // {
      //   source: "/examples/cosmos-kit",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>//developers/typescript/examples/cosmos-kit",
      //   permanent: true,
      // },
      // {
      //   source: "/playground/mixfetch",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/playground/mixfetch",
      //   permanent: true,
      // },
      // {
      //   source: "/playground/traffic",
      //  destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/playground/traffic",
      //   permanent: true,
      // },
      // {
      //   source: "/playground/mixnodes",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/playground/mixnodes",
      //   permanent: true,
      // },
      // {
      //   source: "/playground/wallet",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/typescript/playground/wallet",
      //   permanent: true,
      // },
      // {
      //   source: "/playground/cosmos-kit",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/playground/cosmos-kit",
      //   permanent: true,
      // },
      // {
      //   source: "/bundling/bundling",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/bundling/bundling",
      //   permanent: true,
      // },
      // {
      //   source: "/bundling/esbuild",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/bundling/esbuild",
      //   permanent: true,
      // },
      // {
      //   source: "/bundling/webpack",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/bundling/webpack",
      //   permanent: true,
      // },
      // {
      //   source: "/FAQ/general",
      //   destination: "https://www.<TODO_EDIT_DESTINATION_BASE>/developers/typescript/FAQ",
      //   permanent: true,
      // },
    ];
  },
  images: {
    unoptimized: true,
  },
  transpilePackages: ["@nymproject/contract-clients"],
};

module.exports = config;
