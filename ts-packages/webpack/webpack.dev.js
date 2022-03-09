const { merge } = require('webpack-merge');
const path = require('path');
const common = require('./webpack.common');

/**
 * Creates the default Webpack dev config
 * @param baseDir The base directory path, e.g. pass `__dirname` of the webpack config file using this method
 */
module.exports = (baseDir) =>
  merge(common, {
    mode: 'development',
    entry: path.resolve(baseDir, '/src/index'),
    devServer: {
      port: 9000,
      compress: true,
      historyApiFallback: true,
      hot: true,
      client: {
        overlay: false,
      },
    },
  });
