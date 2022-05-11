module.exports = {
  extends: [
    '@nymproject/eslint-config-react-typescript'
  ],
  overrides: [
    {
      files: ['*.ts'],
      parserOptions: {
        project: 'tsconfig.json',
        tsconfigRootDir: __dirname,
      }
    }
  ]
}
