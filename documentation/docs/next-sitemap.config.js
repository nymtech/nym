/** @type {import('next-sitemap').IConfig} */
module.exports = {
  siteUrl: 'https://nymtech.net/docs',
  generateRobotsTxt: true,
  outDir: './public',
  exclude: ['/api/*', '/docs/_*', '/404'],
  robotsTxtOptions: {
    policies: [
      { userAgent: '*', allow: '/' },
      { userAgent: '*', disallow: ['/api/', '/_next/'] },
    ],
    additionalSitemaps: [
      'https://nymtech.net/docs/sitemap-docs.xml',
    ],
  },
  transform: async (config, path) => ({
    loc: path,
    changefreq: path.includes('/changelog')
      ? 'weekly'
      : path.includes('/docs/operators') || path.includes('/docs/developers')
        ? 'monthly'
        : 'yearly',
    priority: path === '/docs' ? 1.0
      : path.includes('/operators/nodes') || path.includes('/developers') ? 0.8
      : 0.6,
    lastmod: new Date().toISOString(),
  }),
}