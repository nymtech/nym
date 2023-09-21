const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  plugins: [
    new CopyPlugin({
      patterns: [
        {
          from: path.resolve(path.dirname(require.resolve('$PACKAGE/package.json')), '*.wasm'),
          to: '[name][ext]',
        },
        {
          from: path.resolve(path.dirname(require.resolve('$PACKAGE/package.json')), '*worker*.js'),
          to: '[name][ext]',
        },
      ],
    }),
  ],
};
