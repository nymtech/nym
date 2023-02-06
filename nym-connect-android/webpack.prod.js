const path = require('path');
const { default: merge } = require('webpack-merge');
const common = require('./webpack.common');

const entry = {
  app: path.resolve(__dirname, 'src/index.tsx'),
};

module.exports = merge(common, {
  mode: 'production',
  node: {
    __dirname: false,
  },
  entry,
});
