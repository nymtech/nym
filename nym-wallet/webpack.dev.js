const { mergeWithRules } = require('webpack-merge');
const webpack = require('webpack');
const ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');
const ReactRefreshTypeScript = require('react-refresh-typescript');
const commonConfig = require('./webpack.common');

// The mock-wired e2e build (WALLET_MOCK_FAMILIES=on) is driven by Playwright, which reloads
// pages itself — it doesn't need (or want) React Fast Refresh / HMR. Skipping it also avoids
// the HMR client's `core-js-pure` polyfill requirement. See e2e/README.md.
const MOCK_FAMILIES = process.env.WALLET_MOCK_FAMILIES === 'on';

module.exports = mergeWithRules({
  module: {
    rules: {
      test: 'match',
      use: 'replace',
    },
  },
})(commonConfig, {
  mode: 'development',
  devtool: 'inline-source-map',
  // webpack.common sets `resolve.modules` to absolute dirs only, which disables the normal
  // per-module node_modules walk and breaks pnpm's nested symlinks (prop-types → object-assign,
  // webpack-dev-server / react-refresh clients, etc.). Re-add the relative walk.
  resolve: { modules: ['node_modules'] },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
        options: {
          getCustomTransformers: () => ({
            before: MOCK_FAMILIES ? [] : [ReactRefreshTypeScript()],
          }),
          // `ts-loader` does not work with HMR unless `transpileOnly` is used.
          // If you need type checking, `ForkTsCheckerWebpackPlugin` is an alternative.
          transpileOnly: true,
        },
      },
    ],
  },
  plugins: MOCK_FAMILIES
    ? []
    : [
        new ReactRefreshWebpackPlugin(),

        // this can be included automatically by the dev server, however build mode fails if missing
        new webpack.HotModuleReplacementPlugin(),
      ],

  // recommended for faster rebuild
  optimization: {
    runtimeChunk: true,
    removeAvailableModules: false,
    removeEmptyChunks: false,
    splitChunks: false,
  },

  cache: {
    type: 'filesystem',
    buildDependencies: {
      // restart on config change
      config: ['./webpack.dev.js'],
    },
  },

  devServer: {
    port: 9000,
    compress: true,
    historyApiFallback: true,
    // Mock e2e: serve a static bundle only — no HMR, no live-reload client injection
    // (Playwright drives reloads). This also avoids the dev-server client's transitive deps.
    hot: !MOCK_FAMILIES,
    liveReload: !MOCK_FAMILIES,
    webSocketServer: MOCK_FAMILIES ? false : undefined,
    client: MOCK_FAMILIES ? false : { overlay: false },
  },
});
