const { mergeWithRules } = require('webpack-merge');
const webpack = require('webpack');
const ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');
const ReactRefreshTypeScript = require('react-refresh-typescript');
const commonConfig = require('./webpack.common');

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
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
        options: {
          getCustomTransformers: () => ({
            before: [ReactRefreshTypeScript()],
          }),
          // `ts-loader` does not work with HMR unless `transpileOnly` is used.
          // If you need type checking, `ForkTsCheckerWebpackPlugin` is an alternative.
          transpileOnly: true,
        },
      },
    ],
  },
  plugins: [
    new ReactRefreshWebpackPlugin(),

    // this can be included automatically by the dev server, however build mode fails if missing
    new webpack.HotModuleReplacementPlugin(),
    new webpack.ProvidePlugin({
      Buffer: ['buffer', 'Buffer'],
    }),
  ],

  target: 'web',

  devServer: {
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    },
    historyApiFallback: true,
  },

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
      config: ['./webpack.config.js'],
    },
  },
});
