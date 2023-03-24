const path = require('path');
const { default: merge } = require('webpack-merge');
const common = require('./webpack.common');

const entry = {
  app: path.resolve(__dirname, 'src/index.tsx'),
};

const config = merge(common, {
  mode: 'production',
  node: {
    __dirname: false,
  },
  entry,
});

// Remove WebpackFavicons plugin as it makes FDroid build more
// difficult to configure since webpack-favicons depends on sharp,
// which depends on system library libvips
// As we are building for mobile, this is useless anyway
// TODO do not base deletion on index
config.plugins.splice(2, 1);

module.exports = config;
