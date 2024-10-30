const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require("path");

module.exports = {
  performance: {
    hints: false,
    maxEntrypointSize: 512000,
    maxAssetSize: 512000
  },
  entry: {
    bootstrap: "./bootstrap.js"
    // worker: "./worker.js"
  },
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "[name].js"
  },
  mode: "development",
  // mode: 'production',
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        "index.html",
        {
          from: "../../../dist/wasm/nym-credential-proxy-lib-wasm/*.(js|wasm)",
          to: "[name][ext]"
        }
      ]
    })

  ],
  experiments: { syncWebAssembly: true }
};
