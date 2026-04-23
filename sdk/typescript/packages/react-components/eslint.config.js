// @nymproject/react (sdk/typescript/packages/react-components). Legacy sdk/.eslintrc.js shape; airbnb-typescript/base only under **/*.ts (v8 typed rules need project).

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
          '@typescript-eslint/lines-between-class-members': 'off',
          '@typescript-eslint/no-throw-literal': 'off',
          '@typescript-eslint/space-before-function-paren': 'off',
          '@typescript-eslint/no-loss-of-precision': 'off',
          '@typescript-eslint/quotes': 'off',
          '@typescript-eslint/no-empty-object-type': 'off',
          'import/extensions': 'off',
        },
      },
    ],
  }),
  {
    ignores: [
      'tsconfig.json',
      '**/*.d.ts',
      '**/*.tsx',
      '.storybook/**',
      'dist/**',
      'node_modules/**',
      '*.config.js',
    ],
  },
];
