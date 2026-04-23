/**
 * Self-lint config for the shared eslint-config package itself.
 *
 * Intentionally lightweight - this file lints `index.js` (and only that), so
 * we avoid pulling in the full airbnb / typescript-eslint chain that the
 * exported config carries for consumers.
 */

const js = require('@eslint/js');
const prettierPlugin = require('eslint-plugin-prettier');
const prettierConfig = require('eslint-config-prettier');

module.exports = [
  js.configs.recommended,
  {
    files: ['*.js'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'commonjs',
      globals: {
        module: 'readonly',
        require: 'readonly',
        __dirname: 'readonly',
        process: 'readonly',
      },
    },
    plugins: {
      prettier: prettierPlugin,
    },
    rules: {
      ...prettierConfig.rules,
      'prettier/prettier': 'error',
    },
  },
  {
    ignores: ['node_modules/**', 'dist/**'],
  },
];
