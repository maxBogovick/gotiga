export const GAZETTE_MARK_KEYS = [
  'pillar',
  'hive',
  'boom',
  'quill',
  'lens',
  'shard',
  'coil',
  'letter',
] as const;

export type GazetteMarkKey = (typeof GAZETTE_MARK_KEYS)[number];

export function isGazetteMarkKey(value: string | null | undefined): value is GazetteMarkKey {
  return !!value && (GAZETTE_MARK_KEYS as readonly string[]).includes(value);
}

export function markLetter(name: string | null | undefined): string {
  const ch = (name ?? '').trim().charAt(0);
  return ch ? ch.toUpperCase() : '·';
}
