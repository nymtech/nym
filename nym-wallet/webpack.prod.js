const { default: merge } = require('webpack-merge');
const common = require('./webpack.common');

module.exports = merge(common, {
  mode: 'production',
  // Tauri + WKWebView: `publicPath: 'auto'` resolves `__webpack_require__.p` from `document.currentScript`,
  // which is unreliable here and can 404 async chunks so MUI/Emotion never runs (unstyled UI).
  // Relative `./` matches `dist/*.html` + sibling chunk files under the same custom HTTPS origin.
  output: {
    publicPath: './',
  },
  node: {
    __dirname: false,
  },
  optimization: {
    runtimeChunk: 'single',
    splitChunks: {
      chunks: 'all',
      cacheGroups: {
        framework: {
          name: 'framework',
          test: /[\\/]node_modules[\\/](react|react-dom|scheduler|@emotion[\\/](react|styled|cache|sheet|serialize|utils|hash|memoize|weak-memoize|use-insertion-effect-with-fallbacks)|@mui)[\\/]/,
          priority: 40,
          enforce: true,
          reuseExistingChunk: true,
        },
        vendors: {
          test: /[\\/]node_modules[\\/]/,
          priority: 10,
          reuseExistingChunk: true,
        },
      },
    },
  },
});
