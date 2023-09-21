const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('@nymproject/webpack');

const entry = {
  app: path.resolve(__dirname, 'src/index.tsx'),
  log: path.resolve(__dirname, 'src/log.tsx'),
};

module.exports = mergeWithRules({
  module: {
    rules: {
      test: 'match',
      use: 'replace',
    },
  },
})(
  webpackCommon(__dirname, [
    { filename: 'index.html', chunks: ['app'], template: path.resolve(__dirname, 'public/index.html') },
    { filename: 'log.html', chunks: ['log'], template: path.resolve(__dirname, 'public/log.html') },
  ]),
  {
    module: {
      rules: [
        {
          test: /\.mdx?$/,
          use: [
            {
              loader: '@mdx-js/loader',
              /** @type {import('@mdx-js/loader').Options} */
              options: {},
            },
          ],
        },
        {
          test: /\.ya?ml$/,
          type: 'asset/resource',
          use: [
            {
              loader: 'yaml-loader',
              options: {
                asJSON: true,
              },
            },
          ],
        },
      ],
    },
    entry,
    output: {
      clean: true,
      path: path.resolve(__dirname, 'dist'),
      filename: '[name].bundle.js',
      publicPath: '/',
    },
  },
);
