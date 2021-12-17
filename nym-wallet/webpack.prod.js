var path = require('path')
const common = require('./webpack.common')
const { default: merge } = require('webpack-merge')

module.exports = merge(common, {
  mode: 'production',
  node: {
    __dirname: false,
  },
  entry: path.resolve(__dirname, './src/index'),
})
