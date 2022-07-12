const path = require('path');
const { default: merge } = require('webpack-merge');
const common = require('./webpack.common');

module.exports = merge(common, {
  mode: 'production',
  node: {
    __dirname: false,
  },
  entry: path.resolve(__dirname, './src/index'),
});
