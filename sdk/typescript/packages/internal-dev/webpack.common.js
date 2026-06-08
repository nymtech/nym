const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('../../examples/.webpack/webpack.base');

// smolmix-wasm is base64-inlined into the @nymproject/mix-tunnel worker bundle
// (see mix-tunnel/rollup/worker.mjs `maxFileSize`), which is itself base64-inlined
// into mix-tunnel/dist/esm/index.js. No sibling .wasm asset to copy.

module.exports = mergeWithRules({
  module: {
    rules: {
      test: 'match',
      use: 'replace',
    },
  },
})(
  webpackCommon(
    __dirname,
    [
      {
        inject: true,
        filename: 'index.html',
        template: path.resolve(__dirname, 'src/index.html'),
        chunks: ['index'],
      },
    ],
    { skipFavicon: true },
  ),
  {
    entry: {
      index: path.resolve(__dirname, 'src/index.ts'),
    },
    output: {
      path: path.resolve(__dirname, 'dist'),
      publicPath: '/',
    },
  },
);
