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
  resolve: {
    fallback: {
      fs: false,
      tls: false,
      path: false,
      http: false,
      https: false,
      stream: false,
      crypto: false,
      net: false,
      zlib: false,
      buffer: require.resolve('buffer'),
    },
  },
});
