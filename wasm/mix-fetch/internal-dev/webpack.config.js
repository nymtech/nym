const CopyWebpackPlugin = require("copy-webpack-plugin");
const dotenv = require("dotenv").config().parsed;
const path = require("path");
const webpack = require("webpack");

module.exports = {
  performance: {
    hints: false,
    maxEntrypointSize: 512000,
    maxAssetSize: 512000,
  },
  entry: {
    bootstrap: "./bootstrap.js",
    worker: "./worker.js",
  },
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "[name].js",
  },
  mode: "development",
  // mode: 'production',
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        "index.html",
        {
          from: "../pkg/*.(js|wasm)",
          to: "[name][ext]",
        },
        {
          from: "../go-mix-conn/build/*.(js|wasm)",
          to: "[name][ext]",
        },
      ],
    }),
    new webpack.DefinePlugin({
      "process.env.HIDDEN_GATEWAY_OWNER": JSON.stringify(
        dotenv.HIDDEN_GATEWAY_OWNER
      ),
      "process.env.HIDDEN_GATEWAY_EXPLICIT_IP": JSON.stringify(
        dotenv.HIDDEN_GATEWAY_EXPLICIT_IP
      ),
      "process.env.HIDDEN_GATEWAY_HOST": JSON.stringify(
        dotenv.HIDDEN_GATEWAY_HOST
      ),
      "process.env.HIDDEN_GATEWAY_SPHINX_KEY": JSON.stringify(
        dotenv.HIDDEN_GATEWAY_SPHINX_KEY
      ),
      "process.env.HIDDEN_GATEWAY_IDENTITY_KEY": JSON.stringify(
        dotenv.HIDDEN_GATEWAY_IDENTITY_KEY
      ),
      "process.env.PREFFERED_NETWORK_REQUESTER": JSON.stringify(
        dotenv.PREFFERED_NETWORK_REQUESTER
      ),
      "process.env.PREFERRED_GATEWAY": JSON.stringify(dotenv.PREFERRED_GATEWAY),
      "process.env.PREFERRED_VALIDATOR": JSON.stringify(
        dotenv.PREFERRED_VALIDATOR
      ),
    }),
  ],
  experiments: { syncWebAssembly: true },
};
