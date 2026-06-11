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

export const t = derived([lang, copyOverrides], ([$lang, $overrides]) => {
  const d = dicts[$lang];
  const overridesForLang = $overrides[$lang] ?? {};
  return (key: TranslationKey): string =>
    overridesForLang[key] ?? d[key] ?? en[key];
});

export function setLang(l: Lang): void {
  lang.set(l);
}

export function setCopyOverrides(overrides: Record<Lang, Record<string, string>>): void {
  copyOverrides.set(overrides);
}
