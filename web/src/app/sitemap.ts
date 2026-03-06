import type { MetadataRoute } from 'next';

const BASE_URL = process.env.NEXT_PUBLIC_SITE_URL?.trim() || 'https://neuraminds.io';

export default function sitemap(): MetadataRoute.Sitemap {
  const routes = [
    '',
    '/markets',
    '/agents',
    '/portfolio',
    '/wallet',
    '/leaderboard',
    '/settings',
    '/legal',
    '/legal/terms',
    '/legal/privacy',
    '/legal/disclaimer',
    '/docs',
    '/api',
  ];

  const now = new Date();
  return routes.map((path) => ({
    url: `${BASE_URL}${path}`,
    lastModified: now,
    changeFrequency: path === '' || path === '/markets' ? 'hourly' : 'daily',
    priority: path === '' ? 1 : path === '/markets' ? 0.9 : 0.7,
  }));
}
