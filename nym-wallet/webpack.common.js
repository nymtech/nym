const path = require('path');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('@nymproject/webpack');

const entry = {
  auth: path.resolve(__dirname, 'src/auth.tsx'), // JS bundle for sign up/sign in
  main: path.resolve(__dirname, 'src/main.tsx'), // JS bundle for main app
  log: path.resolve(__dirname, 'src/log.tsx'), // JS bundle for logging window
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
    { filename: 'index.html', chunks: ['auth'], template: path.resolve(__dirname, 'public/index.html') }, // the starting point is index.html (sign up/sign in)
    { filename: 'main.html', chunks: ['main'], template: path.resolve(__dirname, 'public/index.html') }, // main app (loaded after sign in in a new window)
    { filename: 'log.html', chunks: ['log'], template: path.resolve(__dirname, 'public/log.html') }, // the user can open a separate logging window
  ]),
  {
    entry,
    output: {
      clean: true,
      path: path.resolve(__dirname, 'dist'),
      filename: '[name].bundle.js',
      publicPath: '/',
    },
    experiments: {
      asyncWebAssembly: true,
    },
  },
);
