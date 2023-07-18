const withNextra = require('nextra')({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.tsx',
});

console.dir(withNextra(), { depth: 30 });
console.dir(withNextra().rewrites, { depth: 30 });

const config = {
  ...withNextra(),
  output: 'export',
  rewrites: undefined,
  images: {
    unoptimized: true,
  },
  basePath: process.env.NODE_ENV === 'development' ? undefined : '/docs/sdk/typescript', // this makes the SDK docs appear at https://nymtech.net/docs/sdk/typescript
};

// config.images.unoptimized = true;

module.exports = config;
