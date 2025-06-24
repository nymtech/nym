import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  trailingSlash: false,

  async rewrites() {
    return [
      // Rewrite /sandbox-explorer to root
      {
        source: "/sandbox-explorer",
        destination: "/",
      },
      // Rewrite /explorer to root
      {
        source: "/explorer",
        destination: "/",
      },
      // Rewrite /sandbox-explorer/* to /*
      {
        source: "/sandbox-explorer/:path*",
        destination: "/:path*",
      },
      // Rewrite /explorer/* to /*
      {
        source: "/explorer/:path*",
        destination: "/:path*",
      },
    ];
  },
};

export default nextConfig;
