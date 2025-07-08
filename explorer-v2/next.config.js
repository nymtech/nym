// @ts-check

/** @type {import('next').NextConfig} */

const { NEXT_PUBLIC_EXPLORER_BASEPATH } = process.env;

const nextConfig = {
    reactStrictMode: true,

    basePath: `/${NEXT_PUBLIC_EXPLORER_BASEPATH}`,
    assetPrefix: `/${NEXT_PUBLIC_EXPLORER_BASEPATH}`,
    trailingSlash: false,

    async redirects() {
        return [
            // Change the basePath to /explorer
            {
                source: "/",
                destination: `/${NEXT_PUBLIC_EXPLORER_BASEPATH}`,
                basePath: false,
                permanent: true,
            },
        ];
    },
};

module.exports = nextConfig