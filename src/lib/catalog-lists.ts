/**
 * Specimen catalog leaf lists (Features / Perfect for).
 *
 * Stored on the figurine as a JSON string (`catalogLists`). Null / omitted
 * means every built-in line is selected and there are no custom lines — that
 * is the default for every work, including existing ones, so nothing needs
 * backfilling.
 */

export const CATALOG_FEATURE_KEYS = [
  'unique',
  'material',
  'technique',
  'handPainted',
  'handFinished',
  'recorded',
  'included',
  'quietRoom',
] as const;

export const CATALOG_PERFECT_KEYS = [
  'collectors',
  'cabinet',
  'looking',
  'closeWork',
  'display',
  'gift',
] as const;

export type CatalogFeatureKey = (typeof CATALOG_FEATURE_KEYS)[number];
export type CatalogPerfectKey = (typeof CATALOG_PERFECT_KEYS)[number];

export interface CatalogCustomLine {
  id: string;
  text: string;
  enabled: boolean;
}

export interface CatalogLists {
  /** Built-in feature keys that are on. Omit / null → all on. `[]` → none. */
  featuresSelected?: string[] | null;
  featuresCustom?: CatalogCustomLine[];
  /** Built-in Perfect-for keys that are on. Omit / null → all on. `[]` → none. */
  perfectSelected?: string[] | null;
  perfectCustom?: CatalogCustomLine[];
}

function asStringArray(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) return undefined;
  return value.filter((item): item is string => typeof item === 'string');
}

function asCustomLines(value: unknown): CatalogCustomLine[] {
  if (!Array.isArray(value)) return [];
  const lines: CatalogCustomLine[] = [];
  for (const item of value) {
    if (!item || typeof item !== 'object') continue;
    const rec = item as Record<string, unknown>;
    const id = typeof rec.id === 'string' && rec.id ? rec.id : newCustomLineId();
    const text = typeof rec.text === 'string' ? rec.text : '';
    const enabled = rec.enabled !== false;
    lines.push({ id, text, enabled });
  }
  return lines;
}

export function parseCatalogLists(raw: string | null | undefined): CatalogLists {
  if (!raw) return {};
  try {
    const value = JSON.parse(raw) as unknown;
    if (!value || typeof value !== 'object') return {};
    const rec = value as Record<string, unknown>;
    const featuresSelected = asStringArray(rec.featuresSelected);
    const perfectSelected = asStringArray(rec.perfectSelected);
    return {
      featuresSelected,
      featuresCustom: asCustomLines(rec.featuresCustom),
      perfectSelected,
      perfectCustom: asCustomLines(rec.perfectCustom),
    };
  } catch {
    return {};
  }
}

function sameKeySet(selected: string[] | null | undefined, all: readonly string[]): boolean {
  if (!selected) return true;
  if (selected.length !== all.length) return false;
  const set = new Set(selected);
  return all.every((key) => set.has(key));
}

export function serialiseCatalogLists(lists: CatalogLists): string | null {
  const featuresCustom = (lists.featuresCustom ?? []).filter((line) => line.id);
  const perfectCustom = (lists.perfectCustom ?? []).filter((line) => line.id);
  const featAll = sameKeySet(lists.featuresSelected, CATALOG_FEATURE_KEYS);
  const perfAll = sameKeySet(lists.perfectSelected, CATALOG_PERFECT_KEYS);
  if (featAll && perfAll && featuresCustom.length === 0 && perfectCustom.length === 0) {
    return null;
  }
  const out: CatalogLists = {};
  if (!featAll) {
    const set = new Set(lists.featuresSelected ?? []);
    out.featuresSelected = CATALOG_FEATURE_KEYS.filter((key) => set.has(key));
  }
  if (featuresCustom.length) out.featuresCustom = featuresCustom;
  if (!perfAll) {
    const set = new Set(lists.perfectSelected ?? []);
    out.perfectSelected = CATALOG_PERFECT_KEYS.filter((key) => set.has(key));
  }
  if (perfectCustom.length) out.perfectCustom = perfectCustom;
  return JSON.stringify(out);
}

/** Null / omitted selected list → every built-in key is on. */
export function isCatalogKeyOn(selected: string[] | null | undefined, key: string): boolean {
  if (selected == null) return true;
  return selected.includes(key);
}

export function toggleCatalogKey(
  selected: string[] | null | undefined,
  key: string,
  all: readonly string[],
): string[] | undefined {
  const current = new Set(selected ?? all);
  if (current.has(key)) current.delete(key);
  else current.add(key);
  const next = all.filter((item) => current.has(item));
  return next.length === all.length ? undefined : next;
}

export function newCustomLineId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `c_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

export function newCustomLine(): CatalogCustomLine {
  return { id: newCustomLineId(), text: '', enabled: true };
}

export function enabledCustomLines(lines: CatalogCustomLine[] | undefined): CatalogCustomLine[] {
  return (lines ?? []).filter((line) => line.enabled && line.text.trim().length > 0);
}
