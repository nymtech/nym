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
      {
        source: "/developers/clients-overview.html",
        destination: "/developers/clients",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/rust.html",
        destination: "/developers/rust",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/message-types.html",
        destination: "/developers/rust/mixnet/message-types",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/message-helpers.html",
        destination: "/developers/rust/mixnet/message-helpers",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/troubleshooting.html",
        destination: "/developers/rust/mixnet/troubleshooting",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples.html",
        destination: "/developers/rust/mixnet/examples",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/simple.html",
        destination: "/developers/rust/mixnet/examples/simple",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/keys.html",
        destination: "/developers/sdk/rust/examples/keys.html",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/storage.html",
        destination:
          "/developers/rust/mixnet/examples/builders/builder-with-storage",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/surbs.html",
        destination: "/developers/rust/mixnet/examples/surbs",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/custom-network.html",
        destination: "/developers/rust/mixnet/examples/custom-topology",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/socks.html",
        destination: "/developers/rust/mixnet/examples/socks",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/split-send.html",
        destination: "/developers/rust/mixnet/examples/split-send",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/credential.html",
        destination: "/developers/rust/mixnet",
        permanent: true,
      },
      {
        source: "/developers/sdk/rust/examples/cargo.html",
        destination: "/developers/rust/importing",
        permanent: true,
      },
      {
        source: "/developers/sdk/typescript.html",
        destination: "/developers/typescript",
        permanent: true,
      },
      {
        source: "/developers/binaries/pre-built-binaries.html",
        destination: "/developers/binaries#pre-built-binaries",
        permanent: true,
      },
      {
        source: "/developers/binaries/building-nym.html",
        destination: "/developers/binaries",
        permanent: true,
      },
      {
        source: "/developers/clients/websocket-client.html",
        destination: "/developers/clients/websocket",
        permanent: true,
      },
      {
        source: "/developers/clients/websocket/setup.html",
        destination: "/developers/clients/websocket/setup",
        permanent: true,
      },
      {
        source: "/developers/clients/websocket/config.html",
        destination: "/developers/clients/websocket/config",
        permanent: true,
      },
      {
        source: "/developers/clients/websocket/usage.html",
        destination: "/developers/clients/websocket/usage",
        permanent: true,
      },
      {
        source: "/developers/clients/websocket/examples.html",
        destination: "/developers/clients/websocket/examples",
        permanent: true,
      },
      {
        source: "/developers/clients/socks5-client.html",
        destination: "/developers/clients/socks5",
        permanent: true,
      },
      {
        source: "/developers/clients/socks5/setup.html",
        destination: "/developers/clients/socks5#client-setup",
        permanent: true,
      },
      {
        source: "/developers/clients/socks5/usage.html",
        destination: "/developers/clients/socks5#using-your-socks5-client",
        permanent: true,
      },
      {
        source: "/developers/clients/webassembly-client.html",
        destination: "/developers/clients/webassembly-client",
        permanent: true,
      },
      {
        source: "/developers/tutorials/coming-soon.html",
        destination: "/developers/rust#",
        permanent: true,
      },
      {
        source: "/developers/integrations/integration-options.html",
        destination: "/developers/integrations",
        permanent: true,
      },
      {
        source: "/developers/faq/integrations-faq.html",
        destination: "/developers/integrations",
        permanent: true,
      },
      {
        source: "/developers/coc.html",
        destination: "/developers/coc",
        permanent: true,
      },
      {
        source: "/developers/licensing.html",
        destination: "/developers/licensing",
        permanent: true,
      },
      {
        source: "/developers/nymvpn/intro.html",
        destination: "/developers/archive/nymvpn",
        permanent: true,
      },
      {
        source: "/developers/nymvpn/cli.html",
        destination: "/developers/nymvpn/cli",
        permanent: true,
      },
      {
        source: "/developers/archive/nym-connect.html",
        destination: "/developers/archive/nym-connect",
        permanent: true,
      },
      {
        source: "/developers/examples/custom-services.html",
        destination: "/developers/rust/mixnet/other-examples#services",
        permanent: true,
      },
      {
        source: "/developers/examples/browser-only.html",
        destination: "/developers/rust/mixnet/other-examples#browser-only",
        permanent: true,
      },
      {
        source: "/developers/examples/using-nrs.html",
        destination: "/developers/rust/mixnet/other-examples",
        permanent: true,
      },
      {
        source: "/developers/examples/monorepo-examples.html",
        destination: "/developers/rust/mixnet/other-examples",
        permanent: true,
      },
      {
        source: "/developers/integrations",
        destination: "/developers/integrations/payment-integration.html",
        permanent: true,
      },
      // {
      //   source: "",
      //   destination: "",
      //   permanent: true,
      // },
      // operators:
      // specific urls that have changed
      {
        source: "/operators/nodes/wallet-preparation.html",
        destination: "/operators/nodes/preliminary-steps/wallet-preparation",
        permanent: true,
      },
      {
        source: "/operators/nodes/vps-setup.html",
        destination: "/operators/nodes/preliminary-steps/vps-setup",
        permanent: true,
      },
      {
        source: "/operators/nodes/proxy-configuration.html",
        destination:
          "/operators/nodes/nym-node/configuration/proxy-configuration",
        permanent: true,
      },
      {
        source: "/operators/nodes/bonding.html",
        destination: "/operators/nodes/nym-node/bonding",
        permanent: true,
      },
      {
        source: "/operators/nodes/nym-api.html",
        destination: "/operators/nodes/validator-setup/nym-api",
        permanent: true,
      },
      {
        source: "/operators/nodes/nyx-configuration.html",
        destination: "/operators/nodes/validator-setup/nyx-configuration",
        permanent: true,
      },
      {
        source: "/operators/nodes/manual-upgrade.html",
        destination: "/operators/nodes/maintenance/manual-upgrade",
        permanent: true,
      },
      {
        source: "/operators/nodes/nymvisor-upgrade.html",
        destination: "/operators/nodes/maintenance/nymvisor-upgrade",
        permanent: true,
      },
      {
        source: "/operators/testing/performance.html",
        destination: "/operators/nodes/performance-and-testing",
        permanent: true,
      },
      {
        source: "/operators/testing/gateway-probe.html",
        destination: "/operators/nodes/performance-and-testing/gateway-probe",
        permanent: true,
      },
      {
        source: "/operators/testing/node-api-check.html",
        destination: "/operators/nodes/performance-and-testing/node-api-check",
        permanent: true,
      },
      {
        source: "/operators/testing/prometheus-grafana.html",
        destination:
          "/operators/nodes/performance-and-testing/prometheus-grafana",
        permanent: true,
      },
      {
        source: "/operators/testing/explorenym-scripts.html",
        destination:
          "/operators/nodes/performance-and-testing/prometheus-grafana/explorenym-scripts",
        permanent: true,
      },
      {
        source: "/operators/legal/community-counsel.html",
        destination: "/operators/community-counsel",
        permanent: true,
      },
      {
        source: "/operators/legal/exit-gateway.html",
        destination: "/operators/community-counsel/exit-gateway",
        permanent: true,
      },
      {
        source: "/operators/legal/isp-list.html",
        destination: "/operators/community-counsel/isp-list",
        permanent: true,
      },
      {
        source: "/operators/legal/jurisdictions.html",
        destination: "/operators/community-counsel/jurisdictions",
        permanent: true,
      },
      {
        source: "/operators/legal/swiss.html",
        destination: "/operators/community-counsel/jurisdictions/swiss",
        permanent: true,
      },
      {
        source: "/operators/legal/united-states.html",
        destination: "/operators/community-counsel/jurisdictions/united-states",
        permanent: true,
      },
      {
        source: "/operators/legal/landing-pages.html",
        destination: "/operators/community-counsel/landing-pages",
        permanent: true,
      },
      {
        source: "/operators/legal/add-content.html",
        destination: "/operators/community-counsel/add-content",
        permanent: true,
      },
      // since the filepaths are mostly the same, we otherwise just match on old URLs that end with .html
      {
        source: "/:path*.html",
        destination: "/:path*",
        permanent: false,
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
