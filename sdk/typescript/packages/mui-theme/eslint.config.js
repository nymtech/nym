/**
 * Flat ESLint config for @nymproject/mui-theme.
 *
 * Mirrors the previous inherited config at sdk/typescript/.eslintrc.js
 * (airbnb-base, NOT airbnb-React). mui-theme has no dedicated tsconfig.eslint
 * - it uses tsconfig.json, matching the previous parent default.
 *
 * Behavior preserved 1:1 with the pre-ESLint-9 setup.
 */

const { FlatCompat } = require('@eslint/eslintrc');
const js = require('@eslint/js');

const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

module.exports = [
  ...compat.config({
    env: {
      browser: true,
      es6: true,
      node: true,
      jest: true,
    },
    parserOptions: {
      ecmaVersion: 2019,
      sourceType: 'module',
    },
    globals: {
      Atomics: 'readonly',
      SharedArrayBuffer: 'readonly',
    },
    plugins: ['prettier'],
    // airbnb-typescript/base is intentionally NOT at the top level - see the
    // matching note in react-components/eslint.config.js for the rationale.
    extends: ['airbnb-base', 'prettier'],
    rules: {
      'prettier/prettier': 'error',
      'import/prefer-default-export': 'off',
      'import/no-extraneous-dependencies': [
        'error',
        {
          devDependencies: ['**/*.test.[jt]s', '**/*.spec.[jt]s'],
        },
      ],
      'import/extensions': [
        'error',
        'ignorePackages',
        {
          ts: 'never',
          js: 'never',
        },
      ],
    },
    overrides: [
      {
        files: ['**/*.ts', '**/*.tsx'],
        parser: '@typescript-eslint/parser',
        parserOptions: {
          project: './tsconfig.json',
          tsconfigRootDir: __dirname,
        },
        plugins: ['@typescript-eslint/eslint-plugin'],
        extends: [
          'airbnb-typescript/base',
          'plugin:@typescript-eslint/eslint-recommended',
          'plugin:@typescript-eslint/recommended',
          'prettier',
        ],
        rules: {
          '@typescript-eslint/explicit-function-return-type': 'off',
          '@typescript-eslint/no-explicit-any': 'off',
          '@typescript-eslint/no-var-requires': 'off',
          'no-use-before-define': [0],
          '@typescript-eslint/no-use-before-define': [1],
          'import/no-unresolved': 0,
          'import/no-extraneous-dependencies': [
            'error',
            {
              devDependencies: ['**/*.test.ts', '**/*.spec.ts'],
            },
          ],
          quotes: [
            2,
            'single',
            {
              avoidEscape: true,
            },
          ],
          '@typescript-eslint/no-unused-vars': [
            2,
            { argsIgnorePattern: '^_', caughtErrors: 'none' },
          ],

          // Rules removed in @typescript-eslint v6/v7/v8 but still referenced
          // by airbnb-typescript@16; disable to avoid unknown-rule errors.
          '@typescript-eslint/lines-between-class-members': 'off',
          '@typescript-eslint/no-throw-literal': 'off',
          '@typescript-eslint/space-before-function-paren': 'off',
          '@typescript-eslint/no-loss-of-precision': 'off',
          '@typescript-eslint/quotes': 'off',

          // New in @typescript-eslint v8 - was previously @typescript-eslint/no-empty-interface
          // and @typescript-eslint/ban-types. Disabled to preserve pre-v8 behavior on
          // existing code (the project relies on `{}` as the default type parameter in some
          // .d.ts type aliases). Address in a follow-up code-cleanup PR.
          '@typescript-eslint/no-empty-object-type': 'off',

          // Pre-existing relative-directory imports rely on Node module resolution
          // (e.g. `import './theme'` resolving to `./theme/index.ts`). The legacy
          // setup did not enforce file extensions for this case either.
          'import/extensions': 'off',
        },
      },
    ],
  }),
  {
    ignores: ['tsconfig.json', '**/*.d.ts', 'dist/**/*', 'dist', 'node_modules', '*.config.js'],
  },
];
