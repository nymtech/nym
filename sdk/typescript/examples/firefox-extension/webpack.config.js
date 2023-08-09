// Webpack configuration for the Firefox extension example

const path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');

module.exports = {
  mode: 'production',
  entry: {
    background: './src/background.js',
    popup: './src/popup.js',
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
  },
  plugins: [
    new CleanWebpackPlugin(),
    new CopyWebpackPlugin({
      patterns: [
        'manifest.json',
        'popup.html',
        { from: path.resolve(__dirname, '../../../../assets/favicon/favicon.png'), to: 'icon.png' },
      ],
    }),
  ],
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
};
