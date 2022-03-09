module.exports = {
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
    project: './tsconfig.json',
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
  ignorePatterns: ['dist/**/*', 'dist', 'node_modules'],
  rules: {
    'jest/prefer-strict-equal': 'error',
    'jest/prefer-to-have-length': 'warn',
    'prettier/prettier': 'error',
    'import/prefer-default-export': 'off',
    'react/prop-types': 'off',
    'react/jsx-filename-extension': 'off',
    'react/jsx-props-no-spreading': 'off',
    'react/require-default-props': [
      1, // warn
      {
        ignoreFunctionalComponents: true,
      },
    ],

    // see https://github.com/facebook/create-react-app/discussions/11864#discussioncomment-1933829
    // and https://github.com/yannickcr/eslint-plugin-react/blob/master/docs/rules/function-component-definition.md
    'react/function-component-definition': [
      2, // error
      {
        namedComponents: 'arrow-function',
        unnamedComponents: 'arrow-function',
      },
    ],

    'import/no-extraneous-dependencies': [
      2, // error
      {
        devDependencies: [
          '**/*.test.[jt]s',
          '**/*.spec.[jt]s',
          '**/*.test.[jt]sx',
          '**/*.spec.[jt]sx',
          // see https://github.com/storybookjs/linter-config/blob/master/eslint.config.js#L100
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
    quotes: 'off',
    '@typescript-eslint/quotes': [
      2,
      'single',
      {
        avoidEscape: true,
      },
    ],
    '@typescript-eslint/no-unused-vars': [2, { argsIgnorePattern: '^_' }],
  },
  // overrides: [
  //   {
  //     files: ['**/*.js', '**/*.jsx'],
  //     parser: 'espree', // override parser to be the default eslint Javascript parser (see https://eslint.org/docs/user-guide/configuring/plugins#specifying-parser)
  //     extends: ['plugin:react/recommended', 'airbnb', 'prettier', 'plugin:jest/recommended', 'plugin:jest/style'],
  //   },
  // ],
  settings: {
    'import/resolver': {
      'root-import': {
        rootPathPrefix: '@',
        rootPathSuffix: 'src',
        extensions: ['.js', '.ts', '.tsx', '.jsx', '.mdx'],
      },
    },
  },
};
