import type { Handle } from '@sveltejs/kit';
import { langFromPath, stripLang, withLang, isLocalizedPath } from '$lib/i18n/path';

/**
 * Prerendered pages paint from HTML+CSS. The 48 `modulepreload` links SvelteKit
 * would otherwise put in `<head>` start fetching JS on the same Slow 4G pipe as
 * the render-blocking stylesheets and the LCP photograph — that was most of the
 * 1.9 s CSS block and the 3.3 s hero download in lab. CSS still preloads;
 * the module graph loads from the single `type="module"` entry after parse.
 */

// Что не является страницей сайта: файлы, служебные пути и прокси. Ссылки на них
// остаются как есть, даже если их начало совпало бы с названием раздела.
const NOT_PAGES = /^\/(_app|static|api|images|fonts|pwa|seals|cdn-cgi)(\/|$)/;

/**
 * Страница на языке читателя (`/ru/…`) должна вести дальше на этом же языке:
 * иначе поисковик, пройдя по ссылкам русской страницы, оказался бы на английской,
 * а читатель — на странице, которая зависит от того, что запомнил браузер.
 * Переписываются только ссылки на страницы, у которых есть версия на каждом
 * языке (`isLocalizedPath`); остальные остаются как были.
 */
function localizeLinks(html: string, l: string): string {
	return html.replace(/(<a\b[^>]*?\shref=")(\/[^"#?]*)([^"]*")/g, (whole, head: string, path: string, tail: string) => {
		if (path.startsWith('//') || NOT_PAGES.test(path)) return whole;
		if (!isLocalizedPath(path)) return whole;
		return `${head}${withLang(path, l)}${tail}`;
	});
}

export const handle: Handle = async ({ event, resolve }) => {
	const res = await resolve(event, {
		preload: ({ type }) => type === 'css' || type === 'font',
	});

	const l = langFromPath(event.url.pathname);
	if (l === 'en' || !res.headers.get('content-type')?.includes('text/html')) return res;
	// `<html lang>` пишется в готовый HTML: у страницы на русском в первом байте
	// уже стоит `ru`, а не английский по умолчанию из app.html.
	if (!isLocalizedPath(stripLang(event.url.pathname))) return res;
	const html = (await res.text())
		.replace(/<html lang="[a-z-]+"/, `<html lang="${l}"`);
	return new Response(localizeLinks(html, l), { status: res.status, statusText: res.statusText, headers: res.headers });
};
