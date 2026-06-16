// Side-effect CSS imports (e.g. `import '@assets/fonts/.../fonts.css'`). Webpack handles the
// actual loading; this just gives the type-checker an ambient module so it doesn't error (TS2882).
declare module '*.css';
