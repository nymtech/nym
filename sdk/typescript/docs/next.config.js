const withNextra = require('nextra')({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.tsx',
});
const { merge } = require('webpack-merge');

console.dir(withNextra(), { depth: 30 });
console.dir(withNextra().rewrites, { depth: 30 });

const config = {
  ...withNextra(),
  webpack: (config, options) => {
    const nextraConfig = withNextra({ webpack: (config) => config });
    const nextraWebpack = nextraConfig.webpack(config, options);

    config.module.rules.push({
      test: /\.txt$/i,
      use: 'raw-loader',
    });
    return Object.assign({}, nextraWebpack, nextraWebpack);
  },
  output: 'export',
  rewrites: undefined,
  images: {
    unoptimized: true,
  },
  transpilePackages: ['@nymproject/contract-clients'],
};

// config.images.unoptimized = true;

module.exports = config;
