const path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const { mergeWithRules } = require('webpack-merge');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');
const { webpackCommon } = require('../.webpack/webpack.base');

module.exports = mergeWithRules({
  module: {
    rules: {
      test: 'match',
      use: 'replace',
    },
  },
})(
  webpackCommon(__dirname, [
    {
      inject: true,
      filename: 'popup.html',
      template: path.resolve(__dirname, 'popup.html'),
      chunks: ['main'],
    },
  ]),

  {
    mode: 'production',
    entry: {
      background: path.resolve(__dirname, './src/background.ts'),
      main: path.resolve(__dirname, './src/popup.ts'),
    },
    output: {
      path: path.resolve(__dirname, 'dist'),
      publicPath: '/',
    },
    plugins: [
      new CleanWebpackPlugin(),
      new CopyWebpackPlugin({
        patterns: [
          'manifest.json',
          { from: path.resolve(__dirname, '../../../../assets/favicon/favicon.png'), to: 'icon.png' },
        ],
      }),
    ],
    resolve: {
      extensions: ['.ts', '.js'],
    },
    module: {
      rules: [
        {
          test: /web-worker.*\.js$/,
          loader: 'worker-loader',
          options: {
            filename: '[name].js',
            inline: 'fallback',
          },
        },
      ],
    },
  },
);
