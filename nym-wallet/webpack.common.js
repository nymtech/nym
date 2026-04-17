const path = require('path');
const webpack = require('webpack');
const { mergeWithRules } = require('webpack-merge');
const { webpackCommon } = require('@nymproject/webpack');

const resolveFromWallet = (request) => require.resolve(request, { paths: [__dirname] });

/** Package root (folder), not main entry - required so `@mui/material/Button` style subpath imports resolve. */
const resolveMuiPackageRoot = (pkg) =>
  path.dirname(require.resolve(`${pkg}/package.json`, { paths: [__dirname, path.resolve(__dirname, '..')] }));

const muiSystemDir = path.dirname(
  require.resolve('@mui/system/package.json', { paths: [__dirname, path.resolve(__dirname, '..')] }),
);
const muiStyledEngineV5 = path.dirname(require.resolve('@mui/styled-engine/package.json', { paths: [muiSystemDir] }));

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
    resolve: {
      // Yarn workspaces hoist deps to ../node_modules; resolve Tauri packages from there too.
      modules: [path.resolve(__dirname, 'node_modules'), path.resolve(__dirname, '../node_modules')],
      alias: {
        // Single Emotion instance so CacheProvider matches MUI's styled engine (workspaces can duplicate).
        '@emotion/react': resolveFromWallet('@emotion/react'),
        '@emotion/styled': resolveFromWallet('@emotion/styled'),
        '@emotion/cache': resolveFromWallet('@emotion/cache'),
        '@mui/styled-engine': muiStyledEngineV5,
        '@mui/material': resolveMuiPackageRoot('@mui/material'),
        '@mui/system': resolveMuiPackageRoot('@mui/system'),
        '@mui/private-theming': resolveMuiPackageRoot('@mui/private-theming'),
        '@mui/utils': resolveMuiPackageRoot('@mui/utils'),
        '@mui/base': resolveMuiPackageRoot('@mui/base'),
        '@mui/styles': resolveMuiPackageRoot('@mui/styles'),
        '@mui/icons-material': resolveMuiPackageRoot('@mui/icons-material'),
        '@mui/lab': resolveMuiPackageRoot('@mui/lab'),
        react$: resolveFromWallet('react'),
        'react-dom$': resolveFromWallet('react-dom'),
        'react-dom/client': resolveFromWallet('react-dom/client'),
        'react/jsx-runtime': resolveFromWallet('react/jsx-runtime'),
        'react/jsx-dev-runtime': resolveFromWallet('react/jsx-dev-runtime'),
      },
    },
    output: {
      clean: true,
      path: path.resolve(__dirname, 'dist'),
      filename: '[name].bundle.js',
      publicPath: '/',
    },
    experiments: {
      asyncWebAssembly: true,
    },
    plugins: [
      new webpack.EnvironmentPlugin({
        NYM_WALLET_INTERNAL_DOCS: '',
      }),
    ],
  },
);
