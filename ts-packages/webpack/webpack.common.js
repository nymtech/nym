const HtmlWebpackPlugin = require('html-webpack-plugin');
const TsconfigPathsPlugin = require('tsconfig-paths-webpack-plugin');
// const { CleanWebpackPlugin } = require('clean-webpack-plugin');
const ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');
const WebpackFavicons = require('webpack-favicons');
const Dotenv = require('dotenv-webpack');
const path = require('path');
const os = require('os');

/**
 * Creates the default Webpack config
 * @param baseDir The base directory path, e.g. pass `__dirname` of the webpack config file using this method
 */
module.exports = (baseDir, htmlPath) => ({
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: [
          {
            loader: 'thread-loader',
            options: { workers: Math.max(2, os.cpus().length - 1) },
          },
          { loader: 'ts-loader', options: { happyPackMode: true } },
        ],
        exclude: /node_modules/,
      },
      {
        test: /\.css$/i,
        use: ['style-loader', 'css-loader'],
      },
      {
        test: /\.svg$/i,
        issuer: /\.[jt]sx?$/,
        use: ['@svgr/webpack'],
      },
      {
        test: /\.(png|jpe?g|gif|md|webp)$/i,
        // More information here https://webpack.js.org/guides/asset-modules/
        type: 'asset',
      },
      {
        // See https://webpack.js.org/guides/asset-management/#loading-fonts
        test: /\.(woff|woff2|eot|ttf|otf)$/i,
        type: 'asset/resource',
      },
      {
        test: /\.ya?ml$/,
        type: 'json',
        use: 'yaml-loader',
      },
    ],
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.js'],
    plugins: [new TsconfigPathsPlugin()],
    alias: {
      'react/jsx-runtime': require.resolve('react/jsx-runtime'),
    },
  },
  plugins: [
    // new CleanWebpackPlugin(),

    ...(Array.isArray(htmlPath)
      ? htmlPath.map((item) => new HtmlWebpackPlugin(item))
      : [
          new HtmlWebpackPlugin({
            filename: 'index.html',
            template: path.resolve(baseDir, htmlPath || 'src/index.html'),
          }),
        ]),

    new ForkTsCheckerWebpackPlugin({
      typescript: {
        mode: 'write-references',
        diagnosticOptions: {
          semantic: true,
          syntactic: true,
        },
      },
    }),

    new WebpackFavicons({
      src: path.resolve(__dirname, '../../assets/favicon/favicon.png'), // the asset directory is relative to THIS file
    }),

    new Dotenv({
      systemvars: true,
    }),
  ],
  output: {
    path: path.resolve(baseDir, 'dist'),
    publicPath: '/',
  },
});
