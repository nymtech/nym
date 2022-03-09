const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('@nymproject/webpack');

module.exports = mergeWithRules({
  module: {
    rules: {
      test: 'match',
      use: 'replace',
    },
  },
})(webpackCommon(__dirname), {
  entry: path.resolve(__dirname, 'src/index.tsx'),
  output: {
    path: path.resolve(__dirname, 'dist'),
    publicPath: '/',
  },
});
