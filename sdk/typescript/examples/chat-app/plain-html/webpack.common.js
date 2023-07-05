const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('../../.webpack/webpack.base');

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
  },
);
