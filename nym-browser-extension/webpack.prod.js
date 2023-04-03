const path = require('path');
const { default: merge } = require('webpack-merge');
const common = require('./webpack.common');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = merge(common, {
  mode: 'production',
  entry: path.resolve(__dirname, 'src/index.tsx'),
  plugins: [
    new CopyPlugin({
      patterns: [
        {
          from: './src/manifest.json',
          to: './',
        },
      ],
    }),
  ],
});
