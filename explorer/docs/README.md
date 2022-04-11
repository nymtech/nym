# Nym Network Explorer - Development Docs

## Getting started

You will need:

- NodeJS
- `nvm`

We use the following:

- Typescript
- `eslint`
- `webpack`
- `jest`
- `react-material-ui`
- `react` 17

## Development mode

Copy the `.env.prod` file to `.env` to configure your environment. Using the live sandbox Explorer API is the best way to do development, so the prod settings are good.

Run the following:

```
npm install
npm run start
```

A development server with hot reloading will be running on http://localhost:3000.

## Linting

`eslint` and `prettier` are configured.

You can lint the code by running:

```
npm run lint
```

> **Note:** this will only show linting errors and will not fix them
 
To fix all linting errors automatically run:

```
npm run lint:fix
```
