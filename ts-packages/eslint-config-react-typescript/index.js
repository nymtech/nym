// ESLint 9 flat preset (FlatCompat). Consumers: `...require('...')` then append `languageOptions.parserOptions`.

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
      ecmaFeatures: {
        jsx: true,
      },
      ecmaVersion: 2019,
      sourceType: 'module',
    },
    globals: {
      Atomics: 'readonly',
      SharedArrayBuffer: 'readonly',
    },
    plugins: ['react', 'react-hooks', 'jsx-a11y', 'prettier', 'jest'],
    extends: [
      'plugin:react/recommended',
      'airbnb',
      'airbnb-typescript',
      'prettier',
      'plugin:jest/recommended',
      'plugin:jest/style',
    ],
    rules: {
      'jest/prefer-strict-equal': 'error',
      'jest/prefer-to-have-length': 'error',
      'prettier/prettier': 'error',
      'import/prefer-default-export': 'off',
      'react/prop-types': 'off',
      'react/jsx-filename-extension': 'off',
      'react/jsx-props-no-spreading': 'off',
      'react/require-default-props': [
        1,
        {
          ignoreFunctionalComponents: true,
        },
      ],
      'react/function-component-definition': [
        2,
        {
          namedComponents: 'arrow-function',
          unnamedComponents: 'arrow-function',
        },
      ],
      'import/no-extraneous-dependencies': [
        2,
        {
          devDependencies: [
            '**/*.test.[jt]s',
            '**/*.spec.[jt]s',
            '**/*.test.[jt]sx',
            '**/*.spec.[jt]sx',
            '**/*.stories.*',
            '**/.storybook/**/*.*',
          ],
          peerDependencies: true,
        },
      ],
      'import/extensions': [
        'error',
        'ignorePackages',
        {
          ts: 'never',
          tsx: 'never',
          js: 'never',
          jsx: 'never',
        },
      ],
      '@typescript-eslint/no-use-before-define': [
        'error',
        { functions: false, classes: true, variables: true, typedefs: true },
      ],
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-var-requires': 'off',
      'no-use-before-define': [0],
      'import/no-unresolved': 0,
      quotes: [
        2,
        'single',
        {
          avoidEscape: true,
        },
      ],
      '@typescript-eslint/no-unused-vars': [2, { argsIgnorePattern: '^_', caughtErrors: 'none' }],
      '@typescript-eslint/lines-between-class-members': 'off',
      '@typescript-eslint/no-throw-literal': 'off',
      '@typescript-eslint/space-before-function-paren': 'off',
      '@typescript-eslint/no-loss-of-precision': 'off',
      '@typescript-eslint/quotes': 'off',
      '@typescript-eslint/no-empty-object-type': 'off',
    },
    settings: {
      'import/resolver': {
        'root-import': {
          rootPathPrefix: '@',
          rootPathSuffix: 'src',
          extensions: ['.js', '.ts', '.tsx', '.jsx', '.mdx'],
        },
      },
    },
  }),
  {
    ignores: ['dist/**', 'node_modules/**'],
  },
];
