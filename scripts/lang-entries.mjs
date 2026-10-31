// Адреса страниц на языках (`/ru/…`, `/de/…`), которые надо пререндерить.
//
// Маршруты у всех языков одни (`src/hooks.ts` отрезает приставку), поэтому сам
// SvelteKit про `/ru/figurines/x` не знает: ссылки он не обходит (`crawl: false`),
// а `entries()` страниц называют только английские адреса. Список составляется
// здесь, из тех же ответов сервера, что у `entries()` страниц работ, газеты и байк.
//
// Приставки повторяют `LANG_PREFIXES` в `src/lib/i18n/path.ts`.
import { STATIC_PAGE_PATHS } from '../src/lib/site-routes.js';

const PREFIXES = ['ru', 'uk', 'es', 'de', 'it'];

async function getJson(base, path) {
	const res = await fetch(`${base}/api/v1${path}`);
	if (!res.ok) throw new Error(`lang-entries: ${path} answered ${res.status}`);
	return res.json();
}

export async function langEntries() {
	if (process.env.VITE_BUILD_TARGET !== 'web') return [];
	const base = (process.env.VITE_API_BASE || '').replace(/\/$/, '');
	if (!base) return [];

	const [figs, gazette, tales, room] = await Promise.all([
		getJson(base, '/figurines?visible=true'),
		getJson(base, '/gazette?page=1&perPage=200'),
		getJson(base, '/tales'),
		getJson(base, '/gazette/blotter').catch(() => ({ years: [] })),
	]);

	const paths = [...STATIC_PAGE_PATHS];
	for (const f of Array.isArray(figs) ? figs : figs.items) paths.push(`/figurines/${f.slug || f.id}`);
	const slugs = new Set();
	for (const leaf of gazette.items) if (!/^\d{4}$/.test(leaf.slug)) slugs.add(leaf.slug);
	for (const year of room.years ?? []) slugs.add(String(year));
	for (const slug of slugs) paths.push(`/gazette/${slug}`);
	for (const tale of tales) paths.push(`/tales/${tale.slug}`);

	return PREFIXES.flatMap((l) => paths.map((p) => (p === '/' ? `/${l}` : `/${l}${p}`)));
}
