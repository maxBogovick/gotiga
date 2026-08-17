import type { FigurineListItem, SemanticHit } from '$lib/types/api';

/** Server already ranks; archive shows this many of that order. */
export const KEEPER_ARCHIVE_MAX = 24;
/** Plates laid on the home blotter. */
export const KEEPER_BLOTTER_MAX = 7;

export type KeeperListFields = Pick<
    FigurineListItem,
    'id' | 'name' | 'shortText' | 'dimensions' | 'material' | 'technique' | 'series' | 'year' | 'status'
>;

/** Light fold for instant substring match. Typos and script mixing are the server's job. */
export function normalizeQuery(s: string): string {
    const lower = s.trim().toLocaleLowerCase().replace(/ё/g, 'е');
    const chars = [...lower];
    let out = '';
    let prevSpace = true;
    for (let i = 0; i < chars.length; i++) {
        const c = chars[i];
        let mapped = c;
        if (c === '×') mapped = 'x';
        else if ((c === 'x' || c === 'х') && neighDigit(chars, i)) mapped = 'x';
        if (/\s/.test(mapped)) {
            if (!prevSpace) {
                out += ' ';
                prevSpace = true;
            }
        } else {
            out += mapped;
            prevSpace = false;
        }
    }
    return out.trim().replace(/(\d)\s*x\s*(?=\d)/g, '$1x');
}

function neighDigit(chars: string[], i: number): boolean {
    let left: string | undefined;
    for (let j = i - 1; j >= 0; j--) {
        if (/\s/.test(chars[j])) continue;
        left = chars[j];
        break;
    }
    let right: string | undefined;
    for (let j = i + 1; j < chars.length; j++) {
        if (/\s/.test(chars[j])) continue;
        right = chars[j];
        break;
    }
    return !!left && !!right && /\d/.test(left) && /\d/.test(right);
}

const IN_HOUSE = new Set([
    'еще в доме',
    'still in the house',
    'in the house',
]);

export function isInHouseQuery(q: string): boolean {
    return IN_HOUSE.has(normalizeQuery(q));
}

function haystacks(f: KeeperListFields): string[] {
    return [
        f.name,
        f.shortText ?? '',
        f.dimensions ?? '',
        f.material ?? '',
        f.technique ?? '',
        f.series ?? '',
        f.year != null ? String(f.year) : '',
    ].map((v) => normalizeQuery(v));
}

/** Instant local filter: same-script substring only. Fuzzy ranking is `/search`. */
export function localMatch<T extends KeeperListFields>(items: T[], query: string): T[] {
    const q = normalizeQuery(query);
    if (!q) return items;
    if (isInHouseQuery(query)) return items.filter((f) => f.status === 'available');
    return items.filter((f) => haystacks(f).some((h) => h.includes(q)));
}

/**
 * Server order first (already fused), then local leftovers the hybrid missed.
 * `serverHits === null` means the keeper has not answered yet — show local only.
 */
export function assembleResults<T extends { id: string }>(
    serverHits: SemanticHit[] | null,
    local: T[],
    byId: Map<string, T>,
    allowed: Set<string>,
    max = KEEPER_ARCHIVE_MAX,
): T[] {
    if (!serverHits) return local;
    const fromServer = serverHits
        .map((h) => byId.get(h.id))
        .filter((f): f is T => !!f && allowed.has(f.id))
        .slice(0, max);
    if (fromServer.length === 0) return local;
    const seen = new Set(fromServer.map((f) => f.id));
    return [...fromServer, ...local.filter((f) => !seen.has(f.id))];
}

export type MatchKind = 'inHouse' | 'name' | 'series' | 'craft' | 'description';

export function matchKind(query: string, f: KeeperListFields): MatchKind {
    if (isInHouseQuery(query) && f.status === 'available') return 'inHouse';
    const q = normalizeQuery(query);
    const name = normalizeQuery(f.name);
    if (name.includes(q)) return 'name';
    const series = normalizeQuery(f.series ?? '');
    if (series && series.includes(q)) return 'series';
    const craft = [f.material, f.technique, f.dimensions]
        .map((v) => normalizeQuery(v ?? ''))
        .some((h) => h && h.includes(q));
    if (craft) return 'craft';
    return 'description';
}

export function whisperSeeds(figurines: FigurineListItem[], lang: 'en' | 'ru'): string[] {
    const series = [
        ...new Set(
            figurines
                .map((f) => f.series?.trim())
                .filter((s): s is string => Boolean(s)),
        ),
    ].slice(0, 2);
    const fixed =
        lang === 'ru'
            ? ['для тёмной библиотеки', 'закрытое лицо', 'ещё в доме', 'со свечой']
            : ['for a dark library', 'a veiled face', 'still in the house', 'holding a candle'];
    return [...series, ...fixed];
}
