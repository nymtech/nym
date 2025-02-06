/** @type {import('next').NextConfig} */
const nextConfig = {
    async redirects() {
        return [
          {
            source: '/network-components/mixnode/:id', // Match the old URL
            destination: '/network-components/nodes/:id', // Redirect to the new URL
            permanent: true, 
          },
        ];
      },
};

export default nextConfig;
