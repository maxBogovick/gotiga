import { writable, derived } from 'svelte/store';
import { en, type TranslationKey } from './en';
import { ru } from './ru';

export type Lang = 'en' | 'ru';
export type { TranslationKey };

const dicts: Record<Lang, Record<TranslationKey, string>> = { en, ru };

function getInitialLang(): Lang {
  if (typeof window === 'undefined') return 'en';
  const saved = localStorage.getItem('gotiga_lang');
  return saved === 'ru' ? 'ru' : 'en';
}

export const lang = writable<Lang>(getInitialLang());

lang.subscribe((l) => {
  if (typeof window !== 'undefined') localStorage.setItem('gotiga_lang', l);
});

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
    const raw = overridesForLang[key] ?? d[key] ?? en[key];
    return raw.includes('{brand}') ? raw.split('{brand}').join(brand) : raw;
  };
});

export function setLang(l: Lang): void {
  lang.set(l);
}

export function setCopyOverrides(overrides: Record<Lang, Record<string, string>>): void {
  copyOverrides.set(overrides);
}
