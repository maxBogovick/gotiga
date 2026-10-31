// Язык в адресе: `/ru/figurines/x`, `/de`, `/it/tales/y`. Английский остаётся без
// приставки (`/figurines/x`) и служит страницей по умолчанию (`x-default`).
//
// Зачем отдельные адреса: поисковик читает адрес без cookie и без localStorage, а
// `?lang=` Google прямо не советует (раздел «Localized versions of your pages»).
// Страницу на каждом языке он может показать только тогда, когда у неё свой
// адрес, свой canonical и ссылки hreflang друг на друга.
//
// Файл без зависимостей от хранилищ: его читают и `hooks.ts` (reroute), и
// сервер, и браузер.
import { SITEMAP_STATIC_ROUTES } from '$lib/site';

export const LANG_PREFIXES = ['ru', 'uk', 'es', 'de', 'it'] as const;
export type LangPrefix = (typeof LANG_PREFIXES)[number];
export type PathLang = 'en' | LangPrefix;

const PREFIX_RE = new RegExp(`^/(${LANG_PREFIXES.join('|')})(?=/|$)`);

/** Язык, названный приставкой адреса; без приставки — английский. */
export function langFromPath(pathname: string): PathLang {
	const m = PREFIX_RE.exec(pathname);
	return m ? (m[1] as LangPrefix) : 'en';
}

/** Адрес без языковой приставки: `/ru/figurines/x` → `/figurines/x`, `/ru` → `/`. */
export function stripLang(pathname: string): string {
	const rest = pathname.replace(PREFIX_RE, '');
	return rest === '' ? '/' : rest;
}

/** Адрес на нужном языке: `('/figurines/x', 'ru')` → `/ru/figurines/x`, `('/', 'de')` → `/de`. */
export function withLang(pathname: string, l: string): string {
	const base = stripLang(pathname);
	if (l === 'en' || !(LANG_PREFIXES as readonly string[]).includes(l)) return base;
	return base === '/' ? `/${l}` : `/${l}${base}`;
}

// Страницы, у которых есть версия на каждом языке: те, что пререндерятся и входят
// в sitemap. Остальные (`/commission`, `/battles/etude`, `/admin`…) остаются
// без приставки: у них нет статического HTML, а читают их уже в браузере.
const STATIC = new Set<string>(SITEMAP_STATIC_ROUTES);
const NOT_LEAVES = new Set(['watch', 'feed.xml']);

/** Есть ли у этого адреса (без приставки) версия на каждом из шести языков. */
export function isLocalizedPath(pathname: string): boolean {
	const base = stripLang(pathname);
	if (STATIC.has(base)) return true;
	const m = /^\/(figurines|gazette|tales)\/([^/.]+)$/.exec(base);
	return !!m && !NOT_LEAVES.has(m[2]);
}
