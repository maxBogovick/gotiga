import { writable, derived, get } from 'svelte/store';
import { en, type TranslationKey as PublicKey } from './en';
import type { AdminTranslationKey } from './en.admin';

export const LANGS = ['en', 'ru', 'uk', 'es', 'de', 'it'] as const;
export type Lang = (typeof LANGS)[number];
export type TranslationKey = PublicKey | AdminTranslationKey;

type Dict = Record<string, string>;

/** Everything a language needs to say about itself, in one place: the three
 *  spellings used by browsers (BCP 47 for `Intl`, `og:locale`, `hreflang`),
 *  the name a reader of that language recognises, and its quotation marks. */
export interface LangMeta {
	/** Name in its own language — the only form a reader can be expected to find. */
	native: string;
	/** Short mark shown on the button. */
	mark: string;
	/** BCP 47 tag for `Intl` and `toLocaleString`. */
	locale: string;
	/** Open Graph locale. */
	og: string;
	/** Opening and closing quotation marks. */
	quotes: [string, string];
}

export const LANG_META: Record<Lang, LangMeta> = {
	en: { native: 'English', mark: 'EN', locale: 'en-US', og: 'en_US', quotes: ['“', '”'] },
	ru: { native: 'Русский', mark: 'RU', locale: 'ru-RU', og: 'ru_RU', quotes: ['«', '»'] },
	uk: { native: 'Українська', mark: 'UA', locale: 'uk-UA', og: 'uk_UA', quotes: ['«', '»'] },
	es: { native: 'Español', mark: 'ES', locale: 'es-ES', og: 'es_ES', quotes: ['«', '»'] },
	de: { native: 'Deutsch', mark: 'DE', locale: 'de-DE', og: 'de_DE', quotes: ['„', '“'] },
	it: { native: 'Italiano', mark: 'IT', locale: 'it-IT', og: 'it_IT', quotes: ['«', '»'] },
};

export function isLang(value: unknown): value is Lang {
	return typeof value === 'string' && (LANGS as readonly string[]).includes(value);
}

const dicts: Record<Lang, Dict> = {
	en: { ...en },
	ru: { ...en },
	uk: { ...en },
	es: { ...en },
	de: { ...en },
	it: { ...en },
};

const loaded: Record<Lang, boolean> = { en: true, ru: false, uk: false, es: false, de: false, it: false };
const loading: Partial<Record<Lang, Promise<void>>> = {};

const loaders: Record<Exclude<Lang, 'en'>, () => Promise<Dict>> = {
	ru: () => import('./ru').then((m) => m.ru),
	uk: () => import('./uk').then((m) => m.uk),
	es: () => import('./es').then((m) => m.es),
	de: () => import('./de').then((m) => m.de),
	it: () => import('./it').then((m) => m.it),
};

/** Brings one language's dictionary into memory. English ships with the page;
 *  the others are fetched on demand so a reader pays only for the one they read. */
export async function loadLang(l: Lang): Promise<void> {
	if (l === 'en' || loaded[l]) return;
	loading[l] ??= loaders[l]().then((d) => {
		dicts[l] = { ...dicts[l], ...d };
		loaded[l] = true;
	});
	await loading[l];
}

/** One dictionary text in a named language, whatever the page is read in —
 *  for records that store every language at once (a new motion's names). */
export async function textInLang(l: Lang, key: PublicKey): Promise<string> {
	await loadLang(l);
	return String((dicts[l] as Record<string, unknown>)[key] ?? '');
}

/** The language a first-time reader gets: an explicit `?lang=`, then what they
 *  chose last time, then what their browser asks for. English is the default
 *  for anything else, as before. */
function getInitialLang(): Lang {
	if (typeof window === 'undefined') return 'en';
	try {
		const asked = new URLSearchParams(window.location.search).get('lang');
		if (isLang(asked)) return asked;
	} catch { /* an unreadable address is not a reason to fail */ }
	try {
		const saved = localStorage.getItem('gotiga_lang');
		if (isLang(saved)) return saved;
	} catch { /* storage may be closed */ }
	const wanted = (navigator.languages?.length ? navigator.languages : [navigator.language]) ?? [];
	for (const raw of wanted) {
		const base = String(raw || '').toLowerCase().split('-')[0];
		if (isLang(base)) return base;
	}
	return 'en';
}

const initialLang = getInitialLang();

function emptyCopy(): Record<Lang, Record<string, string>> {
	return Object.fromEntries(LANGS.map((l) => [l, {}])) as Record<Lang, Record<string, string>>;
}

export const lang = writable<Lang>(initialLang);

lang.subscribe((l) => {
	if (typeof window === 'undefined') return;
	try {
		localStorage.setItem('gotiga_lang', l);
	} catch { /* storage may be closed */ }
});

if (typeof window !== 'undefined' && initialLang !== 'en') {
	void loadLang(initialLang).then(() => lang.update((l) => l));
}

export const copyOverrides = writable<Record<Lang, Record<string, string>>>(emptyCopy());

// Brand name resolution. Precedence:
//   1. admin override (copyOverrides → `brandName` key, per-language, en as fallback)
//   2. compile-time default (VITE_BRAND_NAME, baked in at build)
//   3. hardcoded dictionary default ('Gotiga')
// Anywhere the literal brand appears in copy, use the `{brand}` token — `t()`
// substitutes it below — or read the `brandName` store directly in components.
const ENV_BRAND = (import.meta.env.VITE_BRAND_NAME ?? 'Ritunia').trim();

function resolveBrand($lang: Lang, $overrides: Record<Lang, Record<string, string>>): string {
	return (
		$overrides[$lang]?.brandName?.trim() ||
		$overrides.en?.brandName?.trim() ||
		ENV_BRAND ||
		en.brandName
	);
}

export const brandName = derived(
	[lang, copyOverrides],
	([$lang, $overrides]) => resolveBrand($lang, $overrides),
);

export const t = derived([lang, copyOverrides], ([$lang, $overrides]) => {
	const d = dicts[$lang];
	const overridesForLang = $overrides[$lang] ?? {};
	const brand = resolveBrand($lang, $overrides);
	return (key: TranslationKey): string => {
		const raw = overridesForLang[key] ?? d[key] ?? en[key as PublicKey] ?? String(key);
		return raw.includes('{brand}') ? raw.split('{brand}').join(brand) : raw;
	};
});

/** BCP 47 tag of the language on screen — for dates, numbers, `Intl`. */
export const locale = derived(lang, ($lang) => LANG_META[$lang].locale);

/** The same as `$locale`, for plain functions that format a date when called
 *  and are not part of a component's reactive tree (the admin tables). */
export function currentLocale(): string {
	return LANG_META[get(lang)].locale;
}

export async function setLang(l: Lang): Promise<void> {
	await loadLang(l);
	lang.set(l);
}

export function setCopyOverrides(overrides: Partial<Record<Lang, Record<string, string>>>): void {
	copyOverrides.set({ ...emptyCopy(), ...overrides });
}

/** Merge admin copy into the live dictionaries. Call from the admin route
 *  so the public bundle never ships those strings. */
export function registerAdminDicts(admin: Partial<Record<Lang, Dict>>): void {
	for (const l of LANGS) Object.assign(dicts[l], admin[l] ?? {});
	copyOverrides.update((v) => v);
}

// ── Text that lives in data, not in the dictionaries ───────────────────────

/** Which languages to try, in order, when a text is missing in the one asked
 *  for. A Ukrainian reader is shown English before Russian: an untranslated
 *  line is shown in the world language, not in the language of the aggressor. */
const FALLBACK: Record<Lang, readonly Lang[]> = {
	en: ['en', 'ru', 'uk'],
	ru: ['ru', 'en', 'uk'],
	uk: ['uk', 'en', 'ru'],
	es: ['es', 'en', 'ru', 'uk'],
	de: ['de', 'en', 'ru', 'uk'],
	it: ['it', 'en', 'ru', 'uk'],
};

/** One value per language, for the few places that choose by language in
 *  code (a date format, a quotation mark). A missing language falls back to
 *  English. */
export function pick<T>(l: Lang, values: { en: T } & Partial<Record<Exclude<Lang, 'en'>, T>>): T {
	return values[l] ?? values.en;
}

/** Which language `firstText` would take the text from; nothing — `null`. */
export function firstTextLang(l: Lang, texts: Partial<Record<Lang, string | null | undefined>>): Lang | null {
	for (const k of FALLBACK[l]) if (texts[k]?.trim()) return k;
	return null;
}

/** A name in the quotation marks of the language (`LANG_META.quotes`). */
export function quote(l: Lang, text: string): string {
	const [open, close] = LANG_META[l].quotes;
	return `${open}${text}${close}`;
}

/** The first non-empty text of the list for this language. */
export function firstText(l: Lang, texts: Partial<Record<Lang, string | null | undefined>>): string {
	for (const k of FALLBACK[l]) {
		const v = texts[k]?.trim();
		if (v) return v;
	}
	return '';
}

const cap = (l: Lang) => l[0].toUpperCase() + l.slice(1);

/** A record whose English text sits in the bare field (`title`) and the
 *  other languages beside it with a suffix (`titleRu`, `titleEs`, …). */
export function localizedBase(
	record: object | null | undefined,
	stem: string,
	l: Lang,
): string {
	if (!record) return '';
	const r = record as Record<string, unknown>;
	const texts: Partial<Record<Lang, string>> = {};
	for (const k of LANGS) {
		const v = r[k === 'en' ? stem : stem + cap(k)];
		if (typeof v === 'string') texts[k] = v;
	}
	return firstText(l, texts);
}

/** `titleEn` / `titleRu` / `titleUk` of one record, by field stem. */
export function localized(
	record: object | null | undefined,
	stem: string,
	l: Lang,
): string {
	if (!record) return '';
	const r = record as Record<string, unknown>;
	const texts: Partial<Record<Lang, string>> = {};
	for (const k of LANGS) {
		const v = r[stem + cap(k)];
		if (typeof v === 'string') texts[k] = v;
	}
	return firstText(l, texts);
}

/** A field label written for Ukrainian — «Title (UK)», «Name (uk)» — moved
 *  to another language: the code in brackets is replaced, the word kept. Lets
 *  the admin forms name Spanish, German and Italian fields without a key per
 *  field and language. */
export function labelFor(ukLabel: string, l: Lang): string {
	return ukLabel.replace(/\((uk|UK)\)/, (_m, code: string) => `(${code === 'UK' ? l.toUpperCase() : l})`);
}

/** «Name, Ukrainian» → «Name, Spanish»: the word after the last comma is
 *  swapped for `word`. The studio labels are written that way in every
 *  dictionary. */
export function labelInLang(ukLabel: string, word: string): string {
	return /,[^,]*$/.test(ukLabel) ? ukLabel.replace(/,[^,]*$/, `, ${word}`) : `${ukLabel} · ${word}`;
}

/** Which form a counted word takes. Russian and Ukrainian have three
 *  («1 карта», «2 карты», «5 карт»); English, Spanish, German and Italian two,
 *  «One» only for exactly one, «Many» otherwise. Dictionaries spell the
 *  `…One` / `…Few` / `…Many` keys; a two-form language fills «Few» the same as
 *  «Many». */
export function countForm(l: Lang, n: number): 'One' | 'Few' | 'Many' {
	if (l !== 'ru' && l !== 'uk') return n === 1 ? 'One' : 'Many';
	const ten = n % 10;
	const hundred = n % 100;
	if (ten === 1 && hundred !== 11) return 'One';
	if (ten >= 2 && ten <= 4 && (hundred < 12 || hundred > 14)) return 'Few';
	return 'Many';
}
