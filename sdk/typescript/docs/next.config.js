const withNextra = require('nextra')({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.tsx',
});

console.dir(withNextra(), { depth: 30 });
console.dir(withNextra().rewrites, { depth: 30 });

const nextraConfig = withNextra({
  webpack: (config) => {
    config;
  },
});

const config = {
  ...withNextra(),
  webpack: (config) => {
    return Object.assign(
      {},
      config,
      withNextra({
        webpack: (config) => {
          console.log('config', config);
          return config;
        },
      }),
      {
        externals: Object.assign({}, config.externals, {
          fs: 'fs',
        }),
        module: Object.assign({}, config.module, {
          rules: config.module.rules.concat([
            {
              test: /\.txt$/,
              loader: 'emit-file-loader',
              options: {
                name: 'dist/[path][name].[ext]',
              },
            },
            {
              test: /\.txt$/,
              loader: 'raw-loader',
            },
          ]),
        }),
      },
    );
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
