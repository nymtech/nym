// @ts-check

/** @type {import('next').NextConfig} */
const nextConfig = {
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
        ];
    },
};

module.exports = nextConfig