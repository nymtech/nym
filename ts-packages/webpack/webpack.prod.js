const path = require('path');
const { default: merge } = require('webpack-merge');
const common = require('./webpack.common');

/**
 * Creates the default Webpack prod config
 * @param baseDir The base directory path, e.g. pass `__dirname` of the webpack config file using this method
 */
module.exports = (baseDir) =>
  merge(common, {
    mode: 'production',
    node: {
      __dirname: false,
    },
    module: {
      rules: [
        {
          test: /\.tsx?$/,
          use: [{ loader: 'ts-loader', options: { transpileOnly: true, configFile: 'tsconfig.prod.json' } }],
          exclude: [/node_modules/, '**/*.stories.*', '**/*.test.*'],
        },
      ],
    },
    entry: path.resolve(baseDir, 'src/index'),
  });
