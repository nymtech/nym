const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('@nymproject/webpack');

const entry = {
  index: path.resolve(__dirname, 'src/index.tsx'),
  log: path.resolve(__dirname, 'src/log.tsx'),
};

module.exports = mergeWithRules({
  module: {
    rules: {
      test: 'match',
      use: 'replace',
    },
  },
})(
  webpackCommon(__dirname, [
    { filename: 'index.html', chunks: ['index'], template: 'public/index.html' },
    { filename: 'log.html', chunks: ['log'], template: 'public/log.html' },
  ]),
  {
    entry,
    output: {
      path: path.resolve(__dirname, 'dist'),
      publicPath: '/',
    },
  },
);
