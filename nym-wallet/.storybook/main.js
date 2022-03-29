/* eslint-disable no-param-reassign */
const TsconfigPathsPlugin = require('tsconfig-paths-webpack-plugin');
const ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');

module.exports = {
  stories: ['../src/**/*.stories.mdx', '../src/**/*.stories.@(js|jsx|ts|tsx)'],
  addons: ['@storybook/addon-links', '@storybook/addon-essentials', '@storybook/addon-interactions'],
  framework: '@storybook/react',
  core: {
    builder: 'webpack5',
  },
  // webpackFinal: async (config, { configType }) => {
  //   // `configType` has a value of 'DEVELOPMENT' or 'PRODUCTION'
  //   // You can change the configuration based on that.
  //   // 'PRODUCTION' is used when building the static version of storybook.
  webpackFinal: async (config) => {
    config.module.rules.forEach((rule) => {
      // look for SVG import rule and replace
      // NOTE: the rule before modification is /\.(svg|ico|jpg|jpeg|png|apng|gif|eot|otf|webp|ttf|woff|woff2|cur|ani|pdf)(\?.*)?$/
      if (rule.test?.toString().includes('svg')) {
        rule.test = /\.(ico|jpg|jpeg|png|apng|gif|eot|otf|webp|ttf|woff|woff2|cur|ani|pdf)(\?.*)?$/;
      }
    });

    // handle asset loading with this
    config.module.rules.unshift({
      test: /\.svg(\?.*)?$/i,
      issuer: /\.[jt]sx?$/,
      use: ['@svgr/webpack'],
    });

    config.resolve.extensions = ['.tsx', '.ts', '.js'];
    config.resolve.plugins = [new TsconfigPathsPlugin()];

    config.plugins.push(new ForkTsCheckerWebpackPlugin({
      typescript: {
        mode: 'write-references',
        diagnosticOptions: {
          semantic: true,
          syntactic: true,
        },
      },
    }));

    if (!config.resolve.alias) {
      config.resolve.alias = {};
    }

    config.resolve.alias['@tauri-apps/api'] = `${__dirname}/mocks/tauri`;

    // Return the altered config
    return config;
  },
  features: {
    emotionAlias: false,
  },
};
