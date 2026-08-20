import { writable, derived } from 'svelte/store';
import { en, type TranslationKey as PublicKey } from './en';
import type { AdminTranslationKey } from './en.admin';

export type Lang = 'en' | 'ru';
export type TranslationKey = PublicKey | AdminTranslationKey;

type Dict = Record<string, string>;

const dicts: Record<Lang, Dict> = {
	en: { ...en },
	ru: { ...en },
};

let ruLoaded = false;
let ruLoading: Promise<void> | null = null;

function getInitialLang(): Lang {
	if (typeof window === 'undefined') return 'en';
	const saved = localStorage.getItem('gotiga_lang');
	return saved === 'ru' ? 'ru' : 'en';
}

async function loadRu(): Promise<void> {
	if (ruLoaded) return;
	if (!ruLoading) {
		ruLoading = import('./ru').then((m) => {
			dicts.ru = { ...dicts.ru, ...m.ru };
			ruLoaded = true;
		});
	}
	await ruLoading;
}

export const lang = writable<Lang>(getInitialLang());

lang.subscribe((l) => {
	if (typeof window !== 'undefined') localStorage.setItem('gotiga_lang', l);
});

if (typeof window !== 'undefined' && getInitialLang() === 'ru') {
	void loadRu().then(() => lang.update((l) => l));
}

export const copyOverrides = writable<Record<Lang, Record<string, string>>>({ en: {}, ru: {} });

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

export async function setLang(l: Lang): Promise<void> {
	if (l === 'ru') await loadRu();
	lang.set(l);
}

export function setCopyOverrides(overrides: Record<Lang, Record<string, string>>): void {
	copyOverrides.set(overrides);
}

/** Merge admin copy into the live dictionaries. Call from the admin route
 *  so the public bundle never ships those strings. */
export function registerAdminDicts(enAdmin: Dict, ruAdmin: Dict): void {
	Object.assign(dicts.en, enAdmin);
	Object.assign(dicts.ru, ruAdmin);
	copyOverrides.update((v) => v);
}
