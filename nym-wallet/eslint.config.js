/**
 * Flat ESLint config for @nymproject/nym-wallet-app.
 *
 * Imports the shared React/TS preset and overrides parserOptions.project to
 * point at the wallet's dedicated lint tsconfig.
 */

const sharedConfig = require('@nymproject/eslint-config-react-typescript');

module.exports = [
  ...sharedConfig,
  {
    languageOptions: {
      parserOptions: {
        project: './tsconfig.eslint.json',
        tsconfigRootDir: __dirname,
      },
    },
  },
  {
    ignores: ['dist/**', 'node_modules/**', 'src-tauri/**', '*.config.js'],
  },
];
