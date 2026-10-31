// Страницы без параметров, у которых есть готовый HTML и версия на каждом языке.
// Обычный .js, а не .ts: список читает и `svelte.config.js` (он не умеет TypeScript),
// когда составляет адреса `/ru/…` для пререндера, и `src/lib/site.ts`. Менять здесь.
// Страниц студии (`/studio/gallery` и др.) здесь нет: у них нет готового HTML, и nginx
// отвечает на них кодом 404 (проверено обходом боевого sitemap).
export const STATIC_PAGE_PATHS = [
    '/',
    '/figurines',
    '/gazette',
    '/tales',
    '/workshop',
    '/author',
    '/upcoming',
    '/cellar',
    '/battles',
    '/acquire',
    '/impressions',
    '/privacy',
    '/terms',
    '/rights',
];
