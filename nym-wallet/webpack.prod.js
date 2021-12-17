var path = require('path')

module.exports = {
  mode: 'production',
  node: {
    __dirname: false,
  },
  entry: path.resolve(__dirname, './src/index'),
}
