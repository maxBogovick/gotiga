// Валюта ориентира бюджета в заявке: евро для еврозоны, иначе доллар.
import { get } from 'svelte/store';
import { lang, knownCountry, type Lang } from '$lib/i18n';
import type { BudgetCurrency } from '$lib/requestChoices';

/** Страны, где ходит евро. */
export const EUROZONE = new Set([
  'AT', 'BE', 'HR', 'CY', 'EE', 'FI', 'FR', 'DE', 'GR', 'IE', 'IT',
  'LV', 'LT', 'LU', 'MT', 'NL', 'PT', 'SK', 'SI', 'ES',
]);

/** Сначала страна, если сервер её назвал: из еврозоны — евро. Иначе по языку
 *  сайта: испанский, немецкий, итальянский — евро, остальные — доллар. */
export function budgetCurrency(country: string | null = knownCountry(), l: Lang = get(lang)): BudgetCurrency {
  if (country && EUROZONE.has(country.toUpperCase())) return 'EUR';
  return l === 'es' || l === 'de' || l === 'it' ? 'EUR' : 'USD';
}

export function currencySign(c: BudgetCurrency): string {
  return c === 'EUR' ? '€' : '$';
}
