const common = require('./webpack.common')
const { merge } = require('webpack-merge')
var path = require('path')

module.exports = merge(common, {
  mode: 'development',
  entry: path.resolve(__dirname, '/src/index'),
  devServer: {
    port: 9000,
    compress: true,
    historyApiFallback: true,
    hot: true,
    client: {
      overlay: false,
    },
  },
})
