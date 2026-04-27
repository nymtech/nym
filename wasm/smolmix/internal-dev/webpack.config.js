const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: {
    bundle: './bootstrap.js',
    headless: './headless-bootstrap.js',
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].js',
  },
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: 'index.html', to: '.' },
        { from: 'headless.html', to: '.' },
        { from: '../pkg/smolmix_wasm_bg.wasm', to: '.' },
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
  },
  devServer: {
    static: { directory: path.resolve(__dirname, 'dist') },
    port: 9000,
    // Required for SharedArrayBuffer (if ever needed) and secure context:
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  mode: 'development',
};
