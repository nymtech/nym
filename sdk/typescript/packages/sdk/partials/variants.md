## Find the right package for you

Browsers:
- **`ESM` 🥛** - [@nymproject/sdk](https://www.npmjs.com/package/@nymproject/sdk), not bundled, use `import` syntax, you will need to bundle it
- **`ESM` 🥛🥛🥛** - [@nymproject/sdk-full-fat](https://www.npmjs.com/package/@nymproject/sdk-full-fat), use `import` syntax, pre-bundled with inlined workers and WASM bundles 
- **`CJS` 🥛** - [@nymproject/sdk](https://www.npmjs.com/package/@nymproject/sdk-commonjs), targets ES5, not bundled, you will need to bundle it
- **`CJS` 🥛🥛🥛** - [@nymproject/sdk-full-fat](https://www.npmjs.com/package/@nymproject/sdk-commonjs-full-fat), targets ES5, pre-bundled with inlined workers and WASM bundles

NodeJS:
- **`ESM`** - [@nymproject/sdk-nodejs](https://www.npmjs.com/package/@nymproject/sdk-nodejs), use `import` syntax in NodeJS 18 and later 
- **`CJS`** - [@nymproject/sdk-nodejs-cjs](https://www.npmjs.com/package/@nymproject/sdk-nodejs-cjs), use `require` syntax in older NodeJS versions

Why have all these variations? Each project is different, so hopefully we have something for you!

Choose a package depending on how your project is transpiled and packaged:

- `ESM`: use `import` syntax and have your bundler copy the WASM bundles into your output distribution
- `CJS`: you have an older project that needs ES5 Javascript
- `nodejs`: you want to write your project server-side or locally without the browser on NodeJS

And then, to use `*-full-fat` or not, how do I choose? We have `*-full-fat` packages that are pre-bundled by including all web-workers and WASM as inline Base64.

Use the `*-full-fat` packages when you have trouble changing your bundler settings, or you can use an open CSP.
