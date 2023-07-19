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
      chunks: ['index'],
    },
  ]),
  {
    mode: 'production',
    entry: {
      index: path.resolve(__dirname, '../shared/index.ts'),
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
  },
);
