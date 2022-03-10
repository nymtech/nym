const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('@nymproject/webpack');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const Dotenv = require('dotenv-webpack');

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
  plugins: [
    new HtmlWebpackPlugin({
      template: path.resolve(__dirname, 'public/index.html'),
      filename: 'index.html',
    }),
    new Dotenv(),
  ],
});
