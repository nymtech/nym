/** @type {import('next-sitemap').IConfig} */
module.exports = {
  siteUrl: 'https://nym.com/docs',
  generateRobotsTxt: true,
  outDir: './public',
  exclude: ['/api/*', '/docs/_*', '/404'],
  robotsTxtOptions: {
    policies: [
      { userAgent: '*', allow: '/' },
      { userAgent: '*', disallow: ['/api/', '/_next/'] },
    ],
    additionalSitemaps: [],
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