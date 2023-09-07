const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const CopyPlugin = require('copy-webpack-plugin');
const { webpackCommon } = require('../../../examples/.webpack/webpack.base');

console.log('mix-fetch package path is: ', path.dirname(require.resolve('@nymproject/mix-fetch/package.json')));

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
      filename: 'index.html',
      template: path.resolve(__dirname, 'src/index.html'),
      chunks: ['index'],
    },
  ]),
  {
    entry: {
      index: path.resolve(__dirname, 'src/index.ts'),
    },
    output: {
      path: path.resolve(__dirname, 'dist'),
      publicPath: '/',
    },
    plugins: [
      new CopyPlugin({
        patterns: [
          {
            // copy the WASM files, because webpack doesn't do this automatically even though there are
            // `new URL(..., import.meta.url)` statements in the web worker
            from: path.resolve(path.dirname(require.resolve('@nymproject/mix-fetch/package.json')), 'dist/esm/*.wasm'),
            to: '[name][ext]',
          },
        ],
      }),
    ],
  },
);
