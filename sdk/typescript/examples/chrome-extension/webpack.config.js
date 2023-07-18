// Webpack configuration for the Chrome extension example

const path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');

module.exports = {
  mode: 'production',
  entry: {
    main: './src/main.js',
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
};
