/**
 * Flat ESLint config for @nymproject/react.
 *
 * Mirrors the previous inherited config at sdk/typescript/.eslintrc.js
 * (airbnb-base, NOT airbnb-React) plus the per-package parserOptions override
 * that previously lived in this directory's .eslintrc.json.
 *
 * Behavior preserved 1:1 with the pre-ESLint-9 setup. The fact that React
 * rules are not active here is pre-existing and intentionally not changed in
 * this migration PR.
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
    // airbnb-typescript/base is intentionally NOT at the top level because it
    // bundles type-aware @typescript-eslint rules (e.g. dot-notation, return-await)
    // that throw hard errors on .js files lacking parserOptions.project under
    // typescript-eslint v8. The legacy v5 setup let these rules fail silently;
    // this restructure preserves the effective behavior (no type-aware checks on
    // .js files) without the new hard errors.
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
        // Intentionally NOT including '**/*.tsx'. The legacy eslintrc had
        // `files: '**/*.ts'` (no tsx), and ESLint 8's default extension list
        // was .js-only, so .tsx files were never actually linted. Adding them
        // here would surface a large body of pre-existing issues unrelated to
        // the v9 migration; address in a follow-up dedicated to lint cleanup.
        files: ['**/*.ts'],
        parser: '@typescript-eslint/parser',
        parserOptions: {
          project: './tsconfig.eslint.json',
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
          // existing code. Address in a follow-up code-cleanup PR.
          '@typescript-eslint/no-empty-object-type': 'off',

          // Pre-existing relative-directory imports rely on Node module resolution.
          // The legacy setup did not enforce file extensions for this case either.
          'import/extensions': 'off',
        },
      },
    ],
  }),
  {
    // Skip .tsx files and the .storybook dir to match what the legacy eslintrc
    // setup actually linted in practice (default .js extensions plus the
    // **/*.ts override). The .storybook/*.js files use modern syntax (optional
    // chaining, JSX in .js) that would require a separate parser config; the
    // .tsx files have a pre-existing backlog of lint issues. Both are out of
    // scope for the v9 migration PR.
    ignores: [
      'tsconfig.json',
      '**/*.d.ts',
      '**/*.tsx',
      '.storybook/**',
      'dist/**/*',
      'dist',
      'node_modules',
      '*.config.js',
    ],
  },
];
