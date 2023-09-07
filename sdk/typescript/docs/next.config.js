const withNextra = require('nextra')({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.tsx',
})

console.dir(withNextra(), { depth: 30 });
console.dir(withNextra().rewrites, { depth: 30 });

const config = {
  ...withNextra(),
  output: 'export',
  rewrites: undefined,
  images: {
    unoptimized: true,
  },
  transpilePackages: ['@nymproject/contract-clients']
};

// config.images.unoptimized = true;

module.exports = config;
