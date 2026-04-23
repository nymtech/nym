/**
 * Flat ESLint config for @nymproject/types.
 *
 * Imports the shared React/TS preset and overrides parserOptions.project to
 * point at this package's tsconfig (the shared preset assumes ./tsconfig.json
 * which is what we already use here).
 */

const sharedConfig = require('@nymproject/eslint-config-react-typescript');
const path = require('path');

module.exports = [
  ...sharedConfig,
  {
    languageOptions: {
      parserOptions: {
        project: './tsconfig.json',
        tsconfigRootDir: __dirname,
      },
    },
  },
  {
    ignores: ['dist/**', 'node_modules/**', '*.config.js'],
  },
];
