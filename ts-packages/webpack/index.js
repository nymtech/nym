const webpackDevConfig = require('./webpack.dev');
const webpackProdConfig = require('./webpack.prod');
const webpackCommon = require('./webpack.common');

module.exports = {
  webpackCommon,
  webpackDevConfig,
  webpackProdConfig,
};
