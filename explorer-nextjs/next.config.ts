import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,

  basePath: "/explorer",
  assetPrefix: "/explorer",
  trailingSlash: false,

  async redirects() {
    return [
      // Change the basePath to /explorer
      {
        source: "/",
        destination: "/explorer",
        basePath: false,
        permanent: true,
      },
    ]
  }
};

export default nextConfig;
