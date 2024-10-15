import nextra from "nextra";
// const path = require('path');
// const CopyPlugin = require('copy-webpack-plugin');

const withNextra = nextra({
  theme: "nextra-theme-docs",
  themeConfig: "./theme.config.jsx",
});

const config = {
  // output: 'export', // static HTML files, has problems with Vercel
  // rewrites: undefined,
  images: {
    unoptimized: true,
  },
  transpilePackages: ["@nymproject/contract-clients"],
};

export default withNextra(config);
