// src/lib/api.ts
import type {
    FigurineListItem,
    Figurine,
    AuthorText,
    WorkshopItem,
    ShowingRoom,
    AppSettings,
    ServerRelease,
    AuthorProfile,
    HomeContent,
    OrderRequest,
    OrderMode,
    OrderStatus,
    ReserveStatus,
    MediaInventory,
    MediaCleanupReport,
    MediaReplaceResult,
    FigurineSchedule,
    CreateBookingRequest,
    BookingsPage,
    ShowingDto,
    SaveShowingRequest,
    LoginChallengeResponse,
    LoginVerifyResponse,
    UserDto,
    UserBookingDto,
    UserOrderDto,
    CollectorCertificateDto,
    PublicCertificateDto,
    AdminUsersPage,
    AdminUserDetail,
    ResetTokenResponse,
    CommentDto,
    AdminCommentsPage,
    SubmitCommentRequest,
    ModerateCommentRequest,
    AdminCommentDto,
    ImpressionDto,
    AdminImpressionDto,
    AdminImpressionsPage,
    SubmitImpressionRequest,
    ModerateImpressionRequest,
    AdminLogsPage,
    AdminLogsQuery,
    SmtpSettings,
    ContactSettings,
    ProgrammeSettings,
    BookingRules,
    RescheduleBookingRequest,
    CreateWaitlistRequest,
    WaitlistEntryDto,
    CreateSubscriptionRequest,
    SubscriberDto,
    CreateContactMessageRequest,
    ContactMessageDto,
    MessageThreadDto,
    ThreadMessageDto,
    ThreadDetailDto,
    ThemeConfig,
    BattleCard,
    BattleFrames,
    BattleMe,
    BattleDustRates,
    BattleAttentionResponse,
    BuyBattleCardRequest,
    BuyBattleCardResponse,
    RaiseBattleCardRequest,
    RaiseBattleCardResponse,
    BattleRace,
    SaveBattleRaceRequest,
    BattleKeyword,
    SaveBattleKeywordRequest,
    BattleWeigh,
    BattleChallenge,
    BattleMatches,
    MatchReplay,
    BattleDeck,
    GrantBattleCoinRequest,
    GrantBattleCoinResponse,
    GiveBattleCardsRequest,
    RevokeBattleCardsRequest,
    GiveBattleCardsResponse,
    SaveBattleDeckRequest,
    SaveBattleChallengeRequest,
    BattleMatch,
    BattleAction,
    BenchRequest,
    Bench,
    SaveBattleCardRequest,
    CopyOverrides,
    HomeLayoutConfig,
    HomeLayoutPreset,
    ReelTheme,
    ReelThemePreset,
    DisplayConfigPreset,
    CommissionRequest,
    CommissionDto,
    CommissionCreatedResponse,
    CommissionsPage,
    EditCommissionRequest,
    AttachmentInput,
    DepthGenSummary,
    SemanticHit,
    EmbedIndexSummary,
    BulkOpResult,
    AdminFigurineAnalyticsListPage,
    AdminFigurineAnalyticsDetail,
    AdminAnalyticsOverview,
    CommissionFunnel,
    AnalyticsAnnotation,
    CreateAnnotationRequest,
    LifeOfHouseTrend,
    BackfillAnalyticsRequest,
    BackfillAnalyticsResponse,
    AdminAnalyticsQuery,
    AnalyticsEventPayload,
    GazetteKind,
    GazetteStatus,
    GazetteLeaf,
    GazetteLeavesPage,
    GazetteHome,
    GazetteRoom,
    GazetteCutting,
    GazetteCuttingsPage,
    GazetteFeed,
    SaveGazetteLeafRequest,
    SaveGazetteFeedRequest,
    GazetteRefreshReport,
    WatchGazetteLeafRequest,
    GazetteWatchCreatedResponse,
    GazetteWatchInfo,
    GazetteWatchDto,
} from './types/api';
import { isGazetteReservedSlug } from './gazette';

export type { AppSettings };
export type ImportedMedia = {
    url: string;
    originalUrl?: string | null;
    thumbUrl?: string | null;
};

// --- Web helpers ---
// Cached read of the configured server origin. resolveMediaUrl() reads it once per media
// URL, and AppImage resolves ~8 URLs per image (srcset × jpeg/webp), so an archive grid hit
// localStorage hundreds of times per render for a value that changes only when the user
// saves server settings. Cache it; invalidate on that save (invalidateServerUrlCache) and
// on a cross-tab `storage` event. `null` = not read yet; '' = read, but unset.
let serverUrlCache: string | null = null;
let serverUrlCached = false;
let serverUrlListenerBound = false;
function cachedServerUrl(): string {
    if (typeof localStorage === 'undefined') return '';
    if (!serverUrlCached) {
        serverUrlCache = localStorage.getItem('gotiga_server_url');
        serverUrlCached = true;
        if (typeof window !== 'undefined' && !serverUrlListenerBound) {
            serverUrlListenerBound = true;
            window.addEventListener('storage', (e) => {
                if (e.key === 'gotiga_server_url' || e.key === null) serverUrlCached = false;
            });
        }
    }
    return serverUrlCache ?? '';
}

/** Drop the cached server URL so the next read re-reads localStorage. Call after
 *  writing `gotiga_server_url` in this tab (cross-tab writes invalidate via `storage`). */
export function invalidateServerUrlCache(): void {
    serverUrlCached = false;
}

function getWebSettings(): AppSettings {
    if (typeof localStorage === 'undefined') return { serverUrl: '', apiKey: '' };
    // Admin token may live in sessionStorage when "remember me" is off — it must not
    // outlive the tab. Fall back to it so API calls stay authorized either way.
    const sessionKey = typeof sessionStorage !== 'undefined'
        ? sessionStorage.getItem('gotiga_api_key')
        : null;
    return {
        serverUrl: cachedServerUrl(),
        apiKey: localStorage.getItem('gotiga_api_key') ?? sessionKey ?? '',
    };
}

function getWebHomeContent(): HomeContent {
    if (typeof localStorage === 'undefined') {
        return {
            title: null,
            kicker: null,
            lead: null,
            heroFigurineId: null,
            heroCaptionTitle: null,
            heroCaptionMeta: null,
            heroCaptionCta: null,
            heroMode: null,
            vitrineFigurineId: null,
        };
    }
    return {
        title: localStorage.getItem('gotiga_home_title'),
        kicker: localStorage.getItem('gotiga_home_kicker'),
        lead: localStorage.getItem('gotiga_home_lead'),
        heroFigurineId: localStorage.getItem('gotiga_home_hero_figurine_id'),
        heroCaptionTitle: localStorage.getItem('gotiga_home_hero_caption_title'),
        heroCaptionMeta: localStorage.getItem('gotiga_home_hero_caption_meta'),
        heroCaptionCta: localStorage.getItem('gotiga_home_hero_caption_cta'),
        heroMode: (localStorage.getItem('gotiga_home_hero_mode') as HomeContent['heroMode']) || null,
        vitrineFigurineId: localStorage.getItem('gotiga_home_vitrine_figurine_id'),
    };
}

function webApiBase(): string {
    const { serverUrl } = getWebSettings();
    if (serverUrl) return `${serverUrl}/api/v1`;
    // In the browser always use a same-origin relative path so requests go through the
    // nginx /api proxy. Only during prerender/SSR (Node — no window, no localStorage)
    // do we need an absolute origin, injected at build time via VITE_API_BASE.
    if (typeof window === 'undefined') {
        const buildBase = import.meta.env.VITE_API_BASE;
        if (buildBase) return `${buildBase.replace(/\/$/, '')}/api/v1`;
        // Falling through to the relative path here cannot work: Node's fetch has no
        // origin to resolve it against, so it dies with `Failed to parse URL from
        // /api/v1/…` — thrown from inside undici, surfacing as an unhandled rejection
        // with no hint of the actual cause. Prerender is the ONLY thing that reaches
        // this line, so say what is missing instead.
        throw new Error(
            'VITE_API_BASE is not set. The web build prerenders routes in Node, which ' +
            'needs an absolute API origin to read the catalogue from (a relative ' +
            '/api/v1 path has nothing to resolve against). Set VITE_API_BASE to the ' +
            'API origin, e.g. VITE_API_BASE=https://ritunia.com npm run build:web — or ' +
            'build the SPA target (npm run build) if you did not want prerendering.'
        );
    }
    return '/api/v1';
}

function isLoopbackHost(hostname: string): boolean {
    return hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '0.0.0.0' || hostname === '[::1]';
}

function isAppMediaPath(pathname: string): boolean {
    return pathname.startsWith('/static/') || pathname.startsWith('/api/v1/assets/');
}

/**
 * Resolve a media path (avatar, image, …) to a loadable URL.
 * Relative `/static/` paths are prefixed with the configured server origin in web mode.
 * Shared helper — previously duplicated across SiteHeader/OrderModal/etc.
 *
 * Absolute media URLs (`/static/…`, `/api/v1/assets/…`) are rewritten onto the
 * origin the page can actually load: `gotiga_server_url` when it points at a
 * remote API; a same-origin relative path when both the page and the URL (or
 * the configured server) are loopback. Vite already proxies `/static` to the
 * Rust API, so `http://localhost:3000/static/foo.jpg` on a :1420 UI becomes
 * `/static/foo.jpg`. Production hosts (ritunia.com) are left untouched — the
 * dump may still point there, and the visible photograph does not need CORS
 * to display a foreign image.
 */
export function resolveMediaUrl(url: string | null | undefined): string | null {
    if (!url) return null;
    const value = url.trim();
    if (!value) return null;

    const serverUrl = cachedServerUrl().replace(/\/$/, '');

    if (
        value.startsWith('http://') ||
        value.startsWith('https://')
    ) {
        try {
            const parsed = new URL(value);
            if (isAppMediaPath(parsed.pathname)) {
                const suffix = `${parsed.pathname}${parsed.search}${parsed.hash}`;
                if (serverUrl) {
                    try {
                        const dest = new URL(serverUrl);
                        if (
                            typeof window !== 'undefined'
                            && isLoopbackHost(window.location.hostname)
                            && isLoopbackHost(dest.hostname)
                        ) {
                            return suffix;
                        }
                    } catch {
                        /* prefix as written */
                    }
                    return `${serverUrl}${suffix}`;
                }
                if (
                    typeof window !== 'undefined'
                    && parsed.origin !== window.location.origin
                    && isLoopbackHost(parsed.hostname)
                ) {
                    return suffix;
                }
            }
        } catch {
            return value;
        }
        return value;
    }
    if (value.startsWith('data:') || value.startsWith('blob:') || value.startsWith('file:')) {
        return value;
    }

    if (value.startsWith('//') && typeof location !== 'undefined') {
        return `${location.protocol}${value}`;
    }

    if (value.startsWith('/static/')) {
        return serverUrl ? `${serverUrl}${value}` : value;
    }

    if (value.startsWith('static/')) {
        return serverUrl ? `${serverUrl}/${value}` : `/${value}`;
    }

    if (value.startsWith('/api/v1/assets/')) {
        return serverUrl ? `${serverUrl}${value}` : value;
    }

    // Legacy/local stored media paths, e.g. "images/original/x.jpg" or
    // "backgrounds/x.jpg". In web mode those are served from the API server's
    // static mount, not from the Svelte route.
    if (/^(images|backgrounds|avatars|profile-uploads|uploads)\//.test(value)) {
        return serverUrl ? `${serverUrl}/static/${value}` : `/static/${value}`;
    }

    return value;
}

/**
 * Every uploaded image is written out at three widths — thumb (420px), medium (900px) and
 * preview (1800px) — each as a JPEG and a lossy WebP, all under the SAME uuid
 * (`save_image_variants` in handlers.rs). Only the preview and thumb JPEG paths are ever
 * persisted on the image record, so the rest are derived here by rewriting the directory
 * segment and the extension. That is deliberate: it means adding a rendition costs a
 * directory, not a database migration.
 *
 * The medium rendition is the one that matters on a phone. With only 420 and 1800 to pick
 * from, a device at DPR 2-3 needs ~500-1200 physical pixels, so srcset rightly rejected the
 * 420 thumb as too small and pulled the full 1800 preview — ~390 KB to paint a 390 px-wide
 * screen. 900px lands in that gap.
 */
const VARIANT_DIRS = ['thumb', 'medium', 'preview'] as const;
type Variant = (typeof VARIANT_DIRS)[number];

/** Rewrite an images/{thumb,medium,preview}/{id}.jpg URL to another rendition. */
function resolveVariantUrl(url: string | null | undefined, variant: Variant): string | null {
    const resolved = resolveMediaUrl(url);
    if (!resolved) return null;
    // Only figurine renditions live under images/<variant>/. Anything else (bundled art,
    // avatars, remote URLs) has no siblings here. The main background has its own
    // 900px pair — see resolveBackgroundSrcset.
    const re = new RegExp(`/images/(${VARIANT_DIRS.join('|')})/`);
    if (!re.test(resolved)) return null;
    return resolved.replace(re, `/images/${variant}/`);
}

/**
 * WebP sibling of a JPEG rendition. These are now encoded LOSSY (q80) via libwebp and
 * come in ~25-30% under the JPEG.
 *
 * They were previously encoded losslessly (`WebPEncoder::new_lossless` — the only mode
 * `image` can do), which made them 6-7x LARGER than the JPEG they were meant to undercut:
 * 2480 KB vs 392 KB for a preview, measured on this server's own uploads. Serving those
 * was handing phones megabytes per photograph.
 */
export function resolveWebpUrl(url: string | null | undefined): string | null {
    const resolved = resolveMediaUrl(url);
    if (!resolved) return null;
    if (!/\.jpe?g(\?.*)?$/i.test(resolved)) return null;
    if (!WEBP_SIBLING_PATHS.test(resolved)) return null;
    return resolved.replace(/\.jpe?g(\?.*)?$/i, (_m, q) => `.webp${q ?? ''}`);
}

/**
 * Where a WebP sibling is actually GUARANTEED to exist on disk: the three figurine
 * renditions (save_image_variants) and the main background (process_background_image /
 * backfill_background_image). Nothing else.
 *
 * This guard is not belt-and-braces, it is load-bearing. A `<source type="image/webp">`
 * inside a `<picture>` is chosen on type alone — if the file behind it 404s, the browser
 * does NOT fall back to the `<img>`, it renders a broken image. So a rewrite that guesses
 * at a sibling which was never written (a legacy flat `images/{uuid}.jpg` predating the
 * variant pipeline, a raw file under uploads/, a bundled asset in static/) does not
 * degrade gracefully — it blanks the photo outright.
 */
const WEBP_SIBLING_PATHS = /\/(images\/(thumb|medium|preview)|backgrounds)\//;

/**
 * The full responsive candidate set for an image, or null when the URL is not a figurine
 * rendition (bundled art, avatars, remote) and therefore has no siblings to offer.
 * Widths must match the encoder's constants in handlers.rs.
 */
export function resolveSrcset(url: string | null | undefined): { jpeg: string; webp: string } | null {
    const thumb = resolveVariantUrl(url, 'thumb');
    const medium = resolveVariantUrl(url, 'medium');
    const preview = resolveVariantUrl(url, 'preview');
    if (!thumb || !medium || !preview) return null;

    const jpeg = `${thumb} 420w, ${medium} 900w, ${preview} 1800w`;
    const w = [thumb, medium, preview].map(resolveWebpUrl);
    const webp = w.every(Boolean) ? `${w[0]} 420w, ${w[1]} 900w, ${w[2]} 1800w` : '';
    return { jpeg, webp };
}

/**
 * The admin background is a stable URL (`/static/backgrounds/cabinet-bg.jpg`, or
 * `.jpeg` from older uploads) plus a WebP sibling. It used to be one size — the
 * 1359×822 file was the home LCP element, ~104 KB to paint a ~422×230 phone
 * frame. `process_background_image` now also writes `cabinet-bg-900.{jpg,webp}`
 * (never upscaled). Width descriptors match the encoder caps (900 / 1800).
 *
 * Returns null for the bundled fallback (`/images/cabinet-bg.jpeg`) and for any
 * URL that is not the main background — those have no 900 sibling on disk, and
 * a `<source>` pointing at a 404 blanks the photo (see resolveWebpUrl).
 */
export function resolveBackgroundSrcset(
    url: string | null | undefined,
): { jpeg: string; webp: string } | null {
    const resolved = resolveMediaUrl(url);
    if (!resolved) return null;
    const m = resolved.match(/^(.*\/backgrounds\/)cabinet-bg\.(jpe?g)(\?.*)?$/i);
    if (!m) return null;
    const [, dir, , query = ''] = m;
    const fullWebp = resolveWebpUrl(resolved);
    if (!fullWebp) return null;
    const mediumJpeg = `${dir}cabinet-bg-900.jpg${query}`;
    const mediumWebp = `${dir}cabinet-bg-900.webp${query}`;
    return {
        jpeg: `${mediumJpeg} 900w, ${resolved} 1800w`,
        webp: `${mediumWebp} 900w, ${fullWebp} 1800w`,
    };
}

function webPublicUrl(url: unknown): string | null {
    if (typeof url !== 'string') return null;
    return resolveMediaUrl(url);
}

class ApiError extends Error {
    status: number;

    constructor(status: number, body: string) {
        super(`API ${status}: ${body}`);
        this.name = 'ApiError';
        this.status = status;
    }
}

function isNotFoundError(err: unknown): boolean {
    return err instanceof ApiError && (err.status === 404 || err.status === 410);
}

// fetch selection is architecture-critical (see [[sveltekit-load-global-fetch]] reasoning):
//  • In the BROWSER, prefer the SvelteKit `load` fetch when one is passed — it silences the
//    framework's "window.fetch in load" dev warning and lets SvelteKit dedupe.
//  • During SSR/PRERENDER (no `window`), ALWAYS use the global fetch. The API is a SEPARATE
//    backend reached via the absolute VITE_API_BASE; SvelteKit's server fetch would try to
//    resolve the same-origin /api/v1/* path INTERNALLY → 404 → prerender build abort. The
//    global fetch hits the real backend and a 404 is swallowed by each loader's `.catch()`.
async function webFetch<T>(path: string, options?: RequestInit, loadFetch?: typeof fetch): Promise<T> {
    const doFetch = typeof window !== 'undefined' && loadFetch ? loadFetch : fetch;
    const res = await doFetch(`${webApiBase()}${path}`, options);
    if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new ApiError(res.status, text);
    }
    const ct = res.headers.get('content-type') ?? '';
    if (!ct.includes('json') || res.status === 204) return undefined as T;
    const text = await res.text();
    if (!text) return undefined as T;
    return JSON.parse(text);
}

function authHeaders(): Record<string, string> {
    const { apiKey } = getWebSettings();
    return apiKey ? { Authorization: `Bearer ${apiKey}` } : {};
}

// A handful of read endpoints (main background, author profile) get called
// independently by more than one component on the same page — e.g. SiteHeader
// and the page's own init() both want the author profile. Without this, a
// single page view fires the same GET twice, back-to-back. Short TTL: long
// enough to absorb near-simultaneous callers, short enough that an admin edit
// is never stale for more than a beat.
const readCache = new Map<string, { promise: Promise<unknown>; expires: number }>();

function dedupeRead<T>(key: string, ttlMs: number, fetcher: () => Promise<T>): Promise<T> {
    const cached = readCache.get(key);
    if (cached && cached.expires > Date.now()) return cached.promise as Promise<T>;

    const promise = fetcher();
    readCache.set(key, { promise, expires: Date.now() + ttlMs });
    promise.catch(() => readCache.delete(key));
    return promise;
}

function invalidateRead(key: string): void {
    readCache.delete(key);
}

/** Drop every deduped read whose key starts with `prefix` (e.g. all figurine pages). */
function invalidateReadPrefix(prefix: string): void {
    for (const key of readCache.keys()) {
        if (key.startsWith(prefix)) readCache.delete(key);
    }
}

/**
 * One page of the visible collection, plus the honest `total` behind it. Module-level
 * so the `api` object's own methods can reuse it without going through `this` (callers
 * do destructure `api`).
 */
// The public catalogue is the same for every visitor and changes only on an admin
// edit — which calls invalidateReadPrefix('figurines:') and drops this immediately.
// So the dedupe window can safely match the server's own Cache-Control (max-age=60)
// instead of 4s: every consumer reads the WHOLE catalogue as a lookup table (hero,
// vitrine, marks, prev/next, profile references, the archive), and each route did its
// own fetch, so a 4s window re-pulled the entire collection on essentially every
// navigation. 60s dedups those across a browsing session with no staleness beyond
// what the HTTP layer already permits.
const FIGURINES_PAGE_TTL_MS = 60_000;
async function fetchFigurinesPage(perPage?: number, loadFetch?: typeof fetch): Promise<{ items: FigurineListItem[]; total: number }> {
    return dedupeRead(`figurines:${perPage ?? 'all'}`, FIGURINES_PAGE_TTL_MS, async () => {
        const url = perPage != null
            ? `/figurines?visible=true&perPage=${perPage}`
            : '/figurines?visible=true';
        const res = await webFetch<{ items: FigurineListItem[]; total?: number } | FigurineListItem[]>(url, undefined, loadFetch);
        if (Array.isArray(res)) return { items: res, total: res.length };
        return { items: res.items, total: res.total ?? res.items.length };
    });
}

export function authenticatedApiUrl(path: string): string {
    return `${webApiBase()}${path}`;
}

export function currentAuthHeaders(): Record<string, string> {
    return authHeaders();
}

export const api = {
    // === AUTH ===
    async adminLogin(login: string, password: string): Promise<string> {
        const data = await webFetch<{ token: string }>('/admin/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ login, password }),
        });
        return data.token;
    },

    async adminListLogs(opts?: AdminLogsQuery): Promise<AdminLogsPage> {
        const p = new URLSearchParams();
        const setNumber = (key: string, value: number | null | undefined) => {
            if (value != null && Number.isFinite(value)) p.set(key, String(value));
        };
        setNumber('before_id', opts?.beforeId);
        setNumber('offset', opts?.offset);
        if (opts?.sortBy) p.set('sort_by', opts.sortBy);
        if (opts?.sortDir) p.set('sort_dir', opts.sortDir);
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        if (opts?.level) p.set('level', opts.level);
        if (opts?.requestId) p.set('request_id', opts.requestId);
        if (opts?.route) p.set('route', opts.route);
        if (opts?.method) p.set('method', opts.method);
        setNumber('status', opts?.status);
        setNumber('status_class', opts?.statusClass);
        setNumber('min_latency_ms', opts?.minLatencyMs);
        setNumber('max_latency_ms', opts?.maxLatencyMs);
        if (opts?.target) p.set('target', opts.target);
        if (opts?.q) p.set('q', opts.q);
        setNumber('limit', opts?.limit);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/logs${qs}`, { headers: authHeaders() });
    },

    async backfillAnalytics(req?: BackfillAnalyticsRequest): Promise<BackfillAnalyticsResponse> {
        return webFetch('/admin/analytics/backfill', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req ?? {}),
        });
    },

    async getAnalyticsOverview(opts?: AdminAnalyticsQuery): Promise<AdminAnalyticsOverview> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/overview${qs}`, { headers: authHeaders() });
    },

    async getLifeOfHouseTrend(opts?: AdminAnalyticsQuery): Promise<LifeOfHouseTrend> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/life-of-the-house${qs}`, { headers: authHeaders() });
    },

    async listAnalyticsAnnotations(opts?: AdminAnalyticsQuery): Promise<AnalyticsAnnotation[]> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/annotations${qs}`, { headers: authHeaders() });
    },

    async createAnalyticsAnnotation(req: CreateAnnotationRequest): Promise<AnalyticsAnnotation> {
        return webFetch('/admin/analytics/annotations', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req),
        });
    },

    async deleteAnalyticsAnnotation(id: string): Promise<void> {
        await webFetch(`/admin/analytics/annotations/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async getCommissionFunnel(opts?: AdminAnalyticsQuery): Promise<CommissionFunnel> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/commission-funnel${qs}`, { headers: authHeaders() });
    },

    async getSitePageEngagement(opts?: AdminAnalyticsQuery): Promise<import('./types/api').SitePageEngagementResponse> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/pages${qs}`, { headers: authHeaders() });
    },

    async getVisitorSessions(opts?: { from?: string; to?: string; limit?: number; offset?: number; onlyActions?: boolean }): Promise<import('./types/api').AdminVisitorSessionsPage> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        if (opts?.limit != null) p.set('limit', String(opts.limit));
        if (opts?.offset != null) p.set('offset', String(opts.offset));
        if (opts?.onlyActions) p.set('onlyActions', 'true');
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/visitors${qs}`, { headers: authHeaders() });
    },

    async getVisitorTimeline(visitorHash: string, opts?: AdminAnalyticsQuery): Promise<import('./types/api').AdminVisitorEvent[]> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/visitors/${encodeURIComponent(visitorHash)}${qs}`, { headers: authHeaders() });
    },

    async listFigurineAnalytics(opts?: AdminAnalyticsQuery): Promise<AdminFigurineAnalyticsListPage> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        if (opts?.sort) p.set('sort', opts.sort);
        if (opts?.dir) p.set('dir', opts.dir);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/figurines${qs}`, { headers: authHeaders() });
    },

    async getFigurineAnalytics(id: string, opts?: AdminAnalyticsQuery): Promise<AdminFigurineAnalyticsDetail> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        if (opts?.sort) p.set('sort', opts.sort);
        if (opts?.dir) p.set('dir', opts.dir);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/figurines/${id}${qs}`, { headers: authHeaders() });
    },

    async getFigurineGeoDaily(id: string, opts?: AdminAnalyticsQuery): Promise<import('./types/api').FigurineGeoDailyPoint[]> {
        const p = new URLSearchParams();
        if (opts?.from) p.set('from', opts.from);
        if (opts?.to) p.set('to', opts.to);
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/analytics/figurines/${id}/geo-daily${qs}`, { headers: authHeaders() });
    },

    // Admin-only ranking by mark count (incl. sold/gone pieces). Never rendered publicly.
    async getFigurineMarkStats(): Promise<import('./types/api').AdminFigurineMarkStat[]> {
        return webFetch('/admin/figurine-marks', { headers: authHeaders() });
    },

    async getNoticedByGuestsSettings(): Promise<import('./types/api').NoticedByGuestsSettings> {
        return webFetch('/admin/settings/noticed-by-guests', { headers: authHeaders() });
    },

    async saveNoticedByGuestsSettings(settings: import('./types/api').NoticedByGuestsSettings): Promise<import('./types/api').NoticedByGuestsSettings> {
        return webFetch('/admin/settings/noticed-by-guests', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(settings),
        });
    },

    async sendAnalyticsEvent(payload: AnalyticsEventPayload): Promise<void> {
        await fetch(`${webApiBase()}/analytics/events`, {
            method: 'POST',
            headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
            body: JSON.stringify(payload),
            keepalive: true,
        }).catch(() => undefined);
    },

    // === READ (public) ===

    /**
     * One page of the visible collection, plus the honest `total` behind it.
     *
     * The cap goes out as `perPage` — the name the server's ListParams actually
     * deserializes. It used to be sent as `limit`, which serde silently dropped,
     * leaving `per_page` unset; the service defaults that to `i64::MAX`, so every
     * "limited" read was in fact pulling the ENTIRE catalogue and throwing the
     * tail away on the client. Callers that show a count ("42 pieces catalogued",
     * "26 more in the archive") must read it from `total`, not from `items.length`
     * — the latter is now capped, as it always claimed to be.
     */
    getFigurinesPage: fetchFigurinesPage,

    async getAllFigurines(limit?: number, loadFetch?: typeof fetch): Promise<FigurineListItem[]> {
        const { items } = await fetchFigurinesPage(limit, loadFetch);
        return items;
    },

    async getInProgressFigurines(loadFetch?: typeof fetch): Promise<FigurineListItem[]> {
        return webFetch('/figurines/in-progress', undefined, loadFetch);
    },

    // Works inside their "first look" early-release window — the book-holders'
    // shelf. The home page renders these only for a signed visitor.
    async getFirstLookFigurines(): Promise<FigurineListItem[]> {
        return webFetch('/figurines/first-look');
    },

    // Hybrid curated shelf: admin pins first, remaining slots auto-fill from the
    // private weighted mark ranking. Never carries counts — just the resolved list.
    async getNoticedByGuests(): Promise<FigurineListItem[]> {
        return webFetch('/figurines/noticed');
    },

    async getAllFigurinesAdmin(): Promise<FigurineListItem[]> {
        const res = await webFetch<{ items: FigurineListItem[] } | FigurineListItem[]>('/figurines?visible=false', {
            headers: authHeaders(),
        });
        return Array.isArray(res) ? res : res.items;
    },

    async getFigurine(id: string, loadFetch?: typeof fetch): Promise<Figurine | null> {
        try {
            return await webFetch(`/figurines/${id}`, undefined, loadFetch);
        } catch (e: unknown) {
            if (e instanceof Error && e.message.includes('404')) return null;
            throw e;
        }
    },

    async getAuthorTexts(loadFetch?: typeof fetch): Promise<AuthorText[]> {
        return webFetch('/content/texts/author', undefined, loadFetch);
    },

    async getWorkshopContent(loadFetch?: typeof fetch): Promise<WorkshopItem[]> {
        return webFetch('/content/texts/workshop', undefined, loadFetch);
    },

    async getShowingRooms(loadFetch?: typeof fetch): Promise<ShowingRoom[]> {
        return webFetch('/showing-rooms', undefined, loadFetch);
    },

    // === WRITE (ADMIN) ===
    async saveFigurine(figurine: Figurine): Promise<void> {
        invalidateReadPrefix('figurines:');
        await webFetch('/figurines', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(figurine),
        });
    },

    /**
     * Generate depth maps ("Living Daguerreotype" parallax) for a figurine's
     * images on demand. Runs Depth-Anything in the Rust API (CPU).
     */
    async generateFigurineDepth(id: string): Promise<DepthGenSummary> {
        return webFetch(`/admin/figurines/${id}/generate-depth`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /**
     * Hybrid search ("Хранитель"): rank the archive against a natural-language
     * query (RU or EN) — vector + BM25 + trigrams, fused with RRF. Closest first.
     * An empty array means "no match / query too short", so callers treat []
     * as "keep the current local view".
     */
    async semanticSearch(query: string, limit = 60): Promise<SemanticHit[]> {
        const q = query.trim();
        if (!q) return [];
        return webFetch(`/search?q=${encodeURIComponent(q)}&limit=${limit}`);
    },

    /** Admin: (re)build the "Хранитель" search embeddings for every visible work. */
    async reindexEmbeddings(): Promise<EmbedIndexSummary> {
        return webFetch('/admin/embeddings/reindex', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /**
     * Admin: read a work's backstage visual caption (search-only, never shown to
     * visitors). Written by the offline captioner; returns null when unset.
     */
    async getFigurineCaption(id: string): Promise<string | null> {
        const res = await webFetch<{ caption: string | null }>(`/admin/figurines/${id}/caption`, {
            headers: authHeaders(),
        });
        return res?.caption ?? null;
    },

    /** Admin: set (blank clears) a work's backstage visual caption; re-embeds it. */
    async setFigurineCaption(id: string, caption: string): Promise<void> {
        await webFetch(`/admin/figurines/${id}/caption`, {
            method: 'PUT',
            headers: { ...authHeaders(), 'Content-Type': 'application/json' },
            body: JSON.stringify({ caption }),
        });
    },

    /**
     * Admin: read a work's Pinterest SEO description — feeds feed.xml only,
     * never shown to visitors. Returns null when unset.
     */
    async getFigurinePinterestDescription(id: string): Promise<string | null> {
        const res = await webFetch<{ description: string | null }>(`/admin/figurines/${id}/pinterest-description`, {
            headers: authHeaders(),
        });
        return res?.description ?? null;
    },

    /** Admin: set (blank clears) a work's Pinterest SEO description. */
    async setFigurinePinterestDescription(id: string, description: string): Promise<void> {
        await webFetch(`/admin/figurines/${id}/pinterest-description`, {
            method: 'PUT',
            headers: { ...authHeaders(), 'Content-Type': 'application/json' },
            body: JSON.stringify({ description }),
        });
    },

    async deleteFigurine(id: string): Promise<void> {
        invalidateReadPrefix('figurines:');
        const res = await fetch(`${webApiBase()}/figurines/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === BULK FIGURINE OPS (ADMIN) ===

    /** Clear the manual per-image darkness override for every image, across every figurine. */
    async bulkClearDarkness(): Promise<BulkOpResult> {
        return webFetch('/admin/figurines/bulk/clear-darkness', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /** Reset the manual parallax intensity override for every image, across every figurine. */
    async bulkResetParallax(): Promise<BulkOpResult> {
        return webFetch('/admin/figurines/bulk/reset-parallax', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /** Set the same parallax intensity (0..1) on every image, across every figurine. */
    async bulkSetParallax(intensity: number): Promise<BulkOpResult> {
        return webFetch('/admin/figurines/bulk/set-parallax', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ intensity }),
        });
    },

    // === URL SLUGS (ADMIN) ===

    /** Generate a transliterated URL slug for every work still missing one.
     *  Idempotent — works that already have a slug are untouched. */
    async backfillSlugs(): Promise<BulkOpResult> {
        invalidateReadPrefix('figurines:');
        return webFetch('/admin/figurines/slugs/backfill', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /** Set/regenerate a single work's URL slug. A blank/omitted `slug` regenerates
     *  from the work's name. Returns the slug actually stored (uniqueness-suffixed). */
    async setFigurineSlug(id: string, slug: string | null): Promise<string> {
        invalidateReadPrefix('figurines:');
        const res = await webFetch<{ slug: string }>(`/admin/figurines/${id}/slug`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ slug }),
        });
        return res.slug;
    },

    /**
     * Regenerate depth maps for every image across the whole collection that
     * doesn't have one yet. Server build only, same restriction as
     * `generateFigurineDepth`.
     */
    async bulkRecalculateParallax(): Promise<DepthGenSummary> {
        return webFetch('/admin/figurines/bulk/recalculate-parallax', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /**
     * For every figurine with at least two images, mark the second image
     * (by display order) as the "detail" (second angle) image.
     */
    async bulkSetSecondAngle(): Promise<BulkOpResult> {
        return webFetch('/admin/figurines/bulk/set-second-angle', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    /** Un-feature every figurine on the home page and delete every scheduled showing entry. */
    async bulkClearShowings(): Promise<BulkOpResult> {
        return webFetch('/admin/showings/bulk-clear', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async getMediaInventory(): Promise<MediaInventory> {
        return webFetch('/admin/media', { headers: authHeaders() });
    },

    async getUnusedMediaReport(): Promise<MediaCleanupReport> {
        return webFetch('/admin/media/cleanup-report', { headers: authHeaders() });
    },

    async cleanupReportedUnusedMedia(): Promise<string[]> {
        const data = await webFetch<{ removed: string[] }>('/admin/media/cleanup', {
            method: 'POST',
            headers: authHeaders(),
        });
        return data.removed;
    },

    async replaceMediaEverywhere(oldPath: string, replacementFilePath: string | File): Promise<MediaReplaceResult> {
        const form = new FormData();
        form.append('targetPath', oldPath);
        form.append('file', replacementFilePath as File);
        const res = await fetch(`${webApiBase()}/admin/media/replace`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) throw new Error(`Replace failed: ${res.status}`);
        return res.json();
    },

    // nameHint: for a figurine photo, pass the figurine's name so the server can
    // give the file a readable, keyword-bearing filename instead of a bare UUID —
    // see image_id_with_hint in handlers.rs. Ignored for non-image uploads.
    async importMediaWithVariants(file: File, mediaType: 'images' | 'videos' | 'audio', nameHint?: string): Promise<ImportedMedia> {
        const form = new FormData();
        // Must be appended before `file` — the server reads fields in arrival order
        // and acts on `file` as soon as it sees it (see upload_file in handlers.rs).
        if (nameHint?.trim()) form.append('nameHint', nameHint.trim());
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/upload`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
        const data = await res.json();
        return {
            url: webPublicUrl(data.url) ?? '',
            originalUrl: webPublicUrl(data.originalUrl),
            thumbUrl: webPublicUrl(data.thumbUrl),
        };
    },

    async saveShowingRoom(room: ShowingRoom): Promise<void> {
        await webFetch('/showing-rooms', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(room),
        });
    },

    async deleteShowingRoom(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/showing-rooms/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    async saveText(item: WorkshopItem | AuthorText, category: 'author' | 'workshop'): Promise<void> {
        const dto = {
            id: item.id,
            content: item.content,
            caption: (item as WorkshopItem).caption ?? null,
            imageUrl: (item as WorkshopItem).imageUrl ?? null,
        };
        await webFetch(`/content/texts/${category}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(dto),
        });
    },

    async deleteText(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/content/texts/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    /**
     * The admin-uploaded hero background, or `null` when there is none.
     *
     * It THROWS when the request fails, and that distinction is load-bearing: this used to
     * swallow the error and return `null`, so "there is no background" and "I could not ask"
     * were the same answer. The home page re-reads this after hydration (an admin's new
     * background must not wait for a redeploy), and a caller that cannot tell the two apart
     * responds to one flaky request by clearing the background the page is already showing.
     * Callers that just want a fallback still write `.catch(() => null)`.
     */
    async getMainBackground(loadFetch?: typeof fetch): Promise<string | null> {
        return dedupeRead('main-background', 4000, async () => {
            const data = await webFetch<{ url: string | null }>('/main-background', undefined, loadFetch);
            return data.url;
        });
    },

    // Deduped: the home page's load() and its init() both read this (load() to resolve the
    // hero and the <head> meta, init() so an admin's edit shows up without a rebuild), and
    // without this that is the same GET twice per page view.
    async getHomeContent(loadFetch?: typeof fetch): Promise<HomeContent> {
        return dedupeRead('home-content', 4000, async () => {
            try {
                return await webFetch<HomeContent>('/home-content', undefined, loadFetch);
            } catch {
                return getWebHomeContent();
            }
        });
    },

    async saveHomeContent(content: HomeContent): Promise<void> {
        invalidateRead('home-content');
        try {
            await webFetch('/home-content', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', ...authHeaders() },
                body: JSON.stringify(content),
            });
        } catch {
            if (typeof localStorage === 'undefined') return;
            localStorage.setItem('gotiga_home_title', content.title ?? '');
            localStorage.setItem('gotiga_home_kicker', content.kicker ?? '');
            localStorage.setItem('gotiga_home_lead', content.lead ?? '');
            localStorage.setItem('gotiga_home_hero_figurine_id', content.heroFigurineId ?? '');
            localStorage.setItem('gotiga_home_hero_caption_title', content.heroCaptionTitle ?? '');
            localStorage.setItem('gotiga_home_hero_caption_meta', content.heroCaptionMeta ?? '');
            localStorage.setItem('gotiga_home_hero_caption_cta', content.heroCaptionCta ?? '');
            localStorage.setItem('gotiga_home_hero_mode', content.heroMode ?? '');
            localStorage.setItem('gotiga_home_vitrine_figurine_id', content.vitrineFigurineId ?? '');
        }
    },

    async setMainBackground(fileOrPath: string | File): Promise<string> {
        try {
            const file = fileOrPath as File;
            const form = new FormData();
            form.append('file', file);
            const res = await fetch(`${webApiBase()}/main-background`, {
                method: 'POST',
                headers: authHeaders(),
                body: form,
            });
            if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
            const data = await res.json();
            return data.url as string;
        } finally {
            invalidateRead('main-background');
        }
    },

    // === SYNC & SETTINGS ===
    async getSettings(): Promise<AppSettings> {
        return getWebSettings();
    },

    async saveSettings(settings: AppSettings): Promise<void> {
        localStorage.setItem('gotiga_server_url', settings.serverUrl);
        localStorage.setItem('gotiga_api_key', settings.apiKey);
        invalidateServerUrlCache(); // drop the cached origin so media URLs re-resolve
    },

    async getServerReleases(): Promise<ServerRelease[]> {
        try {
            return await webFetch('/admin/releases', { headers: authHeaders() });
        } catch {
            return [];
        }
    },

    async activateServerRelease(id: string): Promise<void> {
        await webFetch(`/admin/releases/${id}/activate`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async getAuthorProfile(loadFetch?: typeof fetch): Promise<AuthorProfile> {
        return dedupeRead('author-profile', 4000, async () => {
            return webFetch('/author/profile', undefined, loadFetch);
        });
    },

    async saveAuthorProfile(profile: AuthorProfile): Promise<void> {
        try {
            await webFetch('/author/profile', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', ...authHeaders() },
                body: JSON.stringify(profile),
            });
        } finally {
            invalidateRead('author-profile');
        }
    },

    async submitOrder(order: OrderRequest, sessionToken?: string | null): Promise<import('./types/api').OrderCreatedResponse> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
        return webFetch('/orders', {
            method: 'POST',
            headers,
            body: JSON.stringify(order),
        });
    },

    async getNotifyByToken(token: string): Promise<import('./types/api').NotifyInfo | null> {
        try {
            return await webFetch(`/orders/notify/${token}`);
        } catch (err) {
            if (isNotFoundError(err)) return null;
            throw err;
        }
    },

    async cancelNotifyByToken(token: string): Promise<void> {
        await webFetch(`/orders/notify/${token}`, { method: 'POST' });
    },

    async listOrders(opts?: { status?: string; mode?: OrderMode; page?: number; perPage?: number }): Promise<import('./types/api').OrdersPage> {
        const p = new URLSearchParams();
        if (opts?.status)  p.set('status',  opts.status);
        if (opts?.mode)    p.set('mode',    opts.mode);
        if (opts?.page)    p.set('page',    String(opts.page));
        if (opts?.perPage) p.set('perPage', String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/orders${qs}`, { headers: authHeaders() });
    },

    async updateOrderStatus(
        id: string,
        status: OrderStatus,
        opts?: {
            adminNotes?: string | null;
            reserveStatus?: ReserveStatus | null;
            reserveExpiresAt?: string | null;
            adminTermsNote?: string | null;
            invoiceNote?: string | null;
        } | string
    ): Promise<void> {
        const payload = typeof opts === 'string'
            ? { status, adminNotes: opts }
            : { status, ...(opts ?? {}) };
        await webFetch(`/admin/orders/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(payload),
        });
    },

    async issueOrderCertificate(id: string): Promise<CollectorCertificateDto> {
        return webFetch(`/admin/orders/${id}/certificate`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async revokeOrderCertificate(id: string): Promise<CollectorCertificateDto> {
        return webFetch(`/admin/orders/${id}/certificate`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async getPublicCertificate(token: string): Promise<PublicCertificateDto> {
        return webFetch(`/certificates/${encodeURIComponent(token)}`);
    },

    // === SCHEDULE & BOOKINGS (PUBLIC) ===

    async getFigurineSchedule(figurineId: string): Promise<FigurineSchedule> {
        return webFetch(`/figurines/${figurineId}/schedule`);
    },

    async submitBooking(req: CreateBookingRequest, sessionToken?: string | null): Promise<import('./types/api').BookingCreatedResponse> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
        return webFetch(`/figurines/${req.figurineId}/book`, {
            method: 'POST',
            headers,
            body: JSON.stringify(req),
        });
    },

    async getBookingByToken(token: string): Promise<import('./types/api').BookingCancelInfo> {
        return webFetch(`/bookings/cancel/${encodeURIComponent(token)}`);
    },

    // Batch token lookup — one request for many claim tokens. Tokens not found are
    // simply absent from the returned map (same semantics as a 404 on the single GET).
    async getBookingsByTokens(tokens: string[]): Promise<Record<string, import('./types/api').BookingCancelInfo>> {
        if (tokens.length === 0) return {};
        return webFetch('/bookings/by-tokens', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ tokens }),
        });
    },

    async cancelBookingByToken(token: string): Promise<void> {
        await webFetch(`/bookings/cancel/${encodeURIComponent(token)}`, { method: 'POST' });
    },

    async rescheduleBookingByToken(token: string, req: RescheduleBookingRequest): Promise<import('./types/api').BookingCancelInfo> {
        return webFetch(`/bookings/cancel/${encodeURIComponent(token)}/reschedule`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async setFigurineLike(figurineId: string, visitorToken: string, liked: boolean, sessionToken?: string | null): Promise<import('./types/api').LikeToggleResponse> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
        return webFetch(`/figurines/${figurineId}/like`, {
            method: 'POST',
            headers,
            body: JSON.stringify({ visitorToken, liked }),
        });
    },

    // A single quiet wax-seal gesture, not a rating — no count is ever returned here.
    // `tone: null` clears the mark; the caller decides the target state explicitly.
    async toggleFigurineMark(figurineId: string, visitorToken: string, tone: import('./types/api').MarkTone | null): Promise<import('./types/api').MarkToggleResponse> {
        return webFetch(`/figurines/${figurineId}/mark`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ visitorToken, tone }),
        });
    },

    async getBookingRules(): Promise<BookingRules> {
        return webFetch('/booking-rules');
    },

    async saveBookingRules(rules: BookingRules): Promise<void> {
        await webFetch('/admin/booking-rules', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(rules),
        });
    },

    async joinWaitlist(figurineId: string, req: CreateWaitlistRequest): Promise<import('./types/api').WaitlistCreatedResponse> {
        return webFetch(`/figurines/${figurineId}/waitlist`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async getWaitlistByToken(token: string): Promise<import('./types/api').WaitlistCancelInfo | null> {
        try {
            return await webFetch(`/waitlist/leave/${token}`);
        } catch (err) {
            if (isNotFoundError(err)) return null;
            throw err;
        }
    },

    async leaveWaitlistByToken(token: string): Promise<void> {
        await webFetch(`/waitlist/leave/${token}`, { method: 'POST' });
    },

    async adminListWaitlist(figurineId?: string): Promise<WaitlistEntryDto[]> {
        const qs = figurineId ? `?figurineId=${encodeURIComponent(figurineId)}` : '';
        return webFetch(`/admin/waitlist${qs}`, { headers: authHeaders() });
    },

    async adminRemoveFromWaitlist(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/waitlist/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    async adminNotifyWaitlist(figurineId: string): Promise<{ notified: number; total: number }> {
        return webFetch(`/admin/waitlist/${figurineId}/notify`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    // === NEWSLETTER ("visitor book") ===

    async subscribe(req: CreateSubscriptionRequest): Promise<import('./types/api').SubscriptionCreatedResponse> {
        return webFetch('/subscribe', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async getSubscriptionByToken(token: string): Promise<import('./types/api').SubscriberInfo | null> {
        try {
            return await webFetch(`/subscribe/leave/${token}`);
        } catch (err) {
            if (isNotFoundError(err)) return null;
            throw err;
        }
    },

    async unsubscribeByToken(token: string): Promise<void> {
        await webFetch(`/subscribe/leave/${token}`, { method: 'POST' });
    },

    async adminListSubscribers(): Promise<SubscriberDto[]> {
        return webFetch('/admin/subscribers', { headers: authHeaders() });
    },

    async adminRemoveSubscriber(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/subscribers/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === CONTACT MESSAGES ("write to the author") ===

    async submitContactMessage(req: CreateContactMessageRequest): Promise<void> {
        await webFetch('/contact', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async adminListContactMessages(): Promise<ContactMessageDto[]> {
        return webFetch('/admin/contact-messages', { headers: authHeaders() });
    },

    async adminMarkContactMessageRead(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/contact-messages/${id}/read`, {
            method: 'POST',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Mark read failed: ${res.status}`);
    },

    async adminRemoveContactMessage(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/contact-messages/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === SHOWINGS (ADMIN) ===

    async listShowings(): Promise<ShowingDto[]> {
        return webFetch('/admin/showings', { headers: authHeaders() });
    },

    async saveShowing(req: SaveShowingRequest): Promise<ShowingDto> {
        return webFetch('/admin/showings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req),
        });
    },

    async deleteShowing(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/showings/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === BOOKINGS (ADMIN) ===

    async listBookings(opts?: { status?: string; figurineId?: string; page?: number; perPage?: number }): Promise<BookingsPage> {
        const p = new URLSearchParams();
        if (opts?.status)     p.set('status',     opts.status);
        if (opts?.figurineId) p.set('figurineId', opts.figurineId);
        if (opts?.page)       p.set('page',       String(opts.page));
        if (opts?.perPage)    p.set('perPage',     String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/bookings${qs}`, { headers: authHeaders() });
    },

    async updateBookingStatus(id: string, status: string, adminNotes?: string, curatorConditions?: string): Promise<void> {
        await webFetch(`/admin/bookings/${id}/status`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status, adminNotes: adminNotes ?? null, curatorConditions: curatorConditions ?? null }),
        });
    },

    // === USER AUTH ===

    async userRegister(email: string, displayName: string, selections: [string, string, string, string], pool: string[][], ageConfirmed: boolean): Promise<LoginVerifyResponse> {
        return webFetch('/auth/register', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, displayName, selections, pool, ageConfirmed }),
        });
    },

    async userLoginChallenge(email: string): Promise<LoginChallengeResponse> {
        return webFetch('/auth/login/challenge', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email }),
        });
    },

    async userLoginVerify(challengeId: string, tokens: [string, string, string, string]): Promise<LoginVerifyResponse> {
        return webFetch('/auth/login/verify', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ challengeId, tokens }),
        });
    },

    async userLogout(sessionToken: string): Promise<void> {
        await webFetch('/auth/logout', {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userMe(sessionToken: string): Promise<UserDto> {
        return webFetch('/auth/me', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userLinkBookings(sessionToken: string, cancelTokens: string[]): Promise<{ linked: number }> {
        return webFetch('/auth/link-bookings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ cancelTokens }),
        });
    },

    async userProfileBookings(sessionToken: string): Promise<UserBookingDto[]> {
        return webFetch('/profile/bookings', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userProfileOrders(sessionToken: string): Promise<UserOrderDto[]> {
        return webFetch('/profile/orders', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async getWishlist(sessionToken: string): Promise<string[]> {
        return webFetch('/profile/wishlist', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userProfileWaitlist(sessionToken: string): Promise<WaitlistEntryDto[]> {
        return webFetch('/profile/waitlist', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async setWishlist(sessionToken: string, figurineIds: string[]): Promise<string[]> {
        return webFetch('/profile/wishlist', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ figurineIds }),
        });
    },

    // Attach a guest request (booking / waitlist / notify / commission) to the
    // account by its secret code — for visitors who lost the localStorage receipt.
    async linkClaimByToken(sessionToken: string, token: string): Promise<import('./types/api').LinkClaimResponse> {
        return webFetch('/profile/claims/link', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ token }),
        });
    },

    async updateProfile(sessionToken: string, displayName: string): Promise<UserDto> {
        return webFetch('/profile/me', {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ displayName }),
        });
    },

    async uploadAvatar(sessionToken: string, file: File): Promise<UserDto> {
        const form = new FormData();
        form.append('file', file);
        return webFetch('/profile/avatar', {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
            body: form,
        });
    },

    async deleteAccount(sessionToken: string): Promise<void> {
        await webFetch('/profile/me', {
            method: 'DELETE',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    // === ADMIN USER MANAGEMENT ===

    async adminListUsers(opts?: { search?: string; page?: number; perPage?: number }): Promise<AdminUsersPage> {
        const p = new URLSearchParams();
        if (opts?.search)   p.set('search',   opts.search);
        if (opts?.page)     p.set('page',     String(opts.page));
        if (opts?.perPage)  p.set('perPage',  String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/users${qs}`, { headers: authHeaders() });
    },

    async adminGetUser(id: string): Promise<AdminUserDetail> {
        return webFetch(`/admin/users/${id}`, { headers: authHeaders() });
    },

    async adminRevokeUserSessions(id: string): Promise<{ revoked: number }> {
        return webFetch(`/admin/users/${id}/sessions`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async adminUpdateUserNotes(id: string, adminNotes: string | null): Promise<void> {
        await webFetch(`/admin/users/${id}/notes`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ adminNotes }),
        });
    },

    async adminSetUserBlocked(id: string, blocked: boolean): Promise<void> {
        await webFetch(`/admin/users/${id}/block`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ blocked }),
        });
    },

    async adminGenerateResetToken(id: string): Promise<ResetTokenResponse> {
        return webFetch(`/admin/users/${id}/reset-token`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async getUserThreads(sessionToken: string): Promise<{ threads: MessageThreadDto[]; unread: number }> {
        return webFetch('/profile/threads', { headers: { Authorization: `Bearer ${sessionToken}` } });
    },

    async getThread(sessionToken: string, threadId: string): Promise<ThreadDetailDto> {
        return webFetch(`/profile/threads/${threadId}`, { headers: { Authorization: `Bearer ${sessionToken}` } });
    },

    async createThread(sessionToken: string, subject: string, body: string, category?: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadDetailDto> {
        return webFetch('/profile/threads', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ subject, body, category, attachmentUrls }),
        });
    },

    async replyToThread(sessionToken: string, threadId: string, body: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadMessageDto> {
        return webFetch(`/profile/threads/${threadId}/reply`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ body, attachmentUrls }),
        });
    },

    /** Upload an image as a logged-in user (commission refs, chat attachments). */
    async uploadUserMedia(sessionToken: string, file: File): Promise<AttachmentInput> {
        const form = new FormData();
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/profile/uploads`, {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
            body: form,
        });
        if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
        const data = await res.json();
        return {
            url: webPublicUrl(data.url) ?? '',
            thumbUrl: webPublicUrl(data.thumbUrl),
        };
    },

    async adminListThreads(opts?: { category?: string; status?: string; page?: number; perPage?: number }): Promise<{ items: Array<{ thread: MessageThreadDto; user: { id: string; displayName: string; email: string } }>; total: number; page: number; perPage: number }> {
        const p = new URLSearchParams();
        if (opts?.category) p.set('category', opts.category);
        if (opts?.status)   p.set('status',   opts.status);
        if (opts?.page)     p.set('page',     String(opts.page));
        if (opts?.perPage)  p.set('perPage',  String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/threads${qs}`, { headers: authHeaders() });
    },

    async adminGetThread(threadId: string): Promise<ThreadDetailDto> {
        return webFetch(`/admin/threads/${threadId}`, { headers: authHeaders() });
    },

    async adminCreateThreadForUser(userId: string, subject: string, body: string, category?: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadDetailDto> {
        return webFetch(`/admin/users/${userId}/threads`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ subject, body, category, attachmentUrls }),
        });
    },

    async adminReplyToThread(threadId: string, body: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadMessageDto> {
        return webFetch(`/admin/threads/${threadId}/reply`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ body, attachmentUrls }),
        });
    },

    // === COMMISSIONS ===
    async submitCommission(req: CommissionRequest, sessionToken?: string): Promise<CommissionCreatedResponse> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
        return webFetch('/commissions', {
            method: 'POST',
            headers,
            body: JSON.stringify(req),
        });
    },

    async claimCommission(sessionToken: string, claimToken: string): Promise<CommissionDto> {
        return webFetch('/profile/commissions/claim', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ claimToken }),
        });
    },

    async getUserCommissions(sessionToken: string): Promise<CommissionDto[]> {
        const data = await webFetch<{ commissions: CommissionDto[] }>('/profile/commissions', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
        return data.commissions;
    },

    async editCommission(sessionToken: string, id: string, req: EditCommissionRequest): Promise<CommissionDto> {
        return webFetch(`/profile/commissions/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify(req),
        });
    },

    async deleteCommission(sessionToken: string, id: string): Promise<void> {
        await webFetch(`/profile/commissions/${id}`, {
            method: 'DELETE',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async adminDeleteCommission(id: string): Promise<void> {
        await webFetch(`/admin/commissions/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async adminListCommissions(opts?: { status?: string; similar?: boolean; page?: number; perPage?: number }): Promise<CommissionsPage> {
        const p = new URLSearchParams();
        if (opts?.status)  p.set('status',  opts.status);
        if (opts?.similar) p.set('similar', 'true');
        if (opts?.page)    p.set('page',    String(opts.page));
        if (opts?.perPage) p.set('perPage', String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/commissions${qs}`, { headers: authHeaders() });
    },

    async updateCommissionStatus(id: string, status: CommissionDto['status'], opts?: { adminNotes?: string; figurineId?: string }): Promise<CommissionDto> {
        return webFetch(`/admin/commissions/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status, adminNotes: opts?.adminNotes, figurineId: opts?.figurineId }),
        });
    },

    async issueCommissionCertificate(id: string): Promise<CollectorCertificateDto> {
        return webFetch(`/admin/commissions/${id}/certificate`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async revokeCommissionCertificate(id: string): Promise<CollectorCertificateDto> {
        return webFetch(`/admin/commissions/${id}/certificate`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async adminResolveThread(threadId: string): Promise<void> {
        await webFetch(`/admin/threads/${threadId}/resolve`, { method: 'POST', headers: authHeaders() });
    },

    async adminReopenThread(threadId: string): Promise<void> {
        await webFetch(`/admin/threads/${threadId}/reopen`, { method: 'POST', headers: authHeaders() });
    },

    async requestPasswordReset(email: string): Promise<void> {
        await webFetch('/auth/forgot-password', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email }),
        });
    },

    async validateResetToken(token: string): Promise<{ id: string; email: string; displayName: string }> {
        return webFetch(`/auth/reset-token/${token}`);
    },

    async applyPasswordReset(token: string, selections: [string, string, string, string], pool: string[][]): Promise<void> {
        await webFetch('/auth/reset-password', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ token, selections, pool }),
        });
    },

    // === COMMENTS ===

    async getComments(figurineId: string, newestFirst = false): Promise<CommentDto[]> {
        try {
            const qs = newestFirst ? '?sort=newest' : '';
            return await webFetch(`/figurines/${figurineId}/comments${qs}`);
        } catch {
            return [];
        }
    },

    async submitComment(figurineId: string, req: SubmitCommentRequest, sessionToken?: string | null): Promise<void> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers['Authorization'] = `Bearer ${sessionToken}`;
        await webFetch(`/figurines/${figurineId}/comments`, {
            method: 'POST',
            headers,
            body: JSON.stringify(req),
        });
    },

    async adminListComments(opts?: { pending?: boolean; figurineId?: string; sort?: 'newest' | 'oldest'; page?: number; perPage?: number }): Promise<AdminCommentsPage> {
        const p = new URLSearchParams();
        if (opts?.pending)     p.set('pending',    'true');
        if (opts?.figurineId)  p.set('figurineId', opts.figurineId);
        if (opts?.sort)        p.set('sort',        opts.sort);
        if (opts?.page)        p.set('page',        String(opts.page));
        if (opts?.perPage)     p.set('perPage',     String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/comments${qs}`, { headers: authHeaders() });
    },

    async adminModerateComment(id: string, req: ModerateCommentRequest): Promise<AdminCommentDto> {
        return webFetch(`/admin/comments/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req),
        });
    },

    async adminDeleteComment(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/comments/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === IMPRESSIONS ("Book of Impressions") ===

    async submitImpression(req: SubmitImpressionRequest): Promise<void> {
        await webFetch('/impressions', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async getFeaturedImpressions(): Promise<ImpressionDto[]> {
        try {
            return await webFetch('/impressions/featured');
        } catch {
            return [];
        }
    },

    async adminListImpressions(opts?: { pending?: boolean; sort?: 'newest' | 'oldest'; page?: number; perPage?: number }): Promise<AdminImpressionsPage> {
        const p = new URLSearchParams();
        if (opts?.pending) p.set('pending', 'true');
        if (opts?.sort)    p.set('sort',    opts.sort);
        if (opts?.page)    p.set('page',    String(opts.page));
        if (opts?.perPage) p.set('perPage', String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/impressions${qs}`, { headers: authHeaders() });
    },

    async adminModerateImpression(id: string, req: ModerateImpressionRequest): Promise<AdminImpressionDto> {
        return webFetch(`/admin/impressions/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req),
        });
    },

    async adminDeleteImpression(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/impressions/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === SMTP SETTINGS ===

    async getSmtpSettings(): Promise<SmtpSettings> {
        return webFetch('/admin/settings/smtp', { headers: authHeaders() });
    },

    async saveSmtpSettings(s: SmtpSettings): Promise<SmtpSettings> {
        return webFetch('/admin/settings/smtp', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(s),
        });
    },

    // === CONTACT SETTINGS ===

    async getContactSettings(): Promise<ContactSettings> {
        return webFetch('/settings/contact');
    },

    async saveContactSettings(s: ContactSettings): Promise<ContactSettings> {
        return webFetch('/admin/settings/contact', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(s),
        });
    },

    // === WORKSHOP FEATURE (home page) ===

    async getProgrammeSettings(): Promise<ProgrammeSettings> {
        return webFetch('/settings/programme');
    },

    async saveProgrammeSettings(settings: ProgrammeSettings): Promise<ProgrammeSettings> {
        return webFetch('/admin/settings/programme', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(settings),
        });
    },

    // === THEME CONFIG ===

    async getThemeConfig(): Promise<ThemeConfig> {
        return webFetch('/settings/theme');
    },

    async saveThemeConfig(config: ThemeConfig): Promise<ThemeConfig> {
        return webFetch('/admin/settings/theme', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(config),
        });
    },

    // === REEL THEME ===

    async getReelTheme(): Promise<ReelTheme> {
        return webFetch('/settings/reel-theme');
    },

    async saveReelTheme(config: ReelTheme): Promise<ReelTheme> {
        return webFetch('/admin/settings/reel-theme', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(config),
        });
    },

    async getReelThemePresets(): Promise<ReelThemePreset[]> {
        return webFetch('/admin/settings/reel-theme-presets', { headers: authHeaders() });
    },

    async saveReelThemePresets(presets: ReelThemePreset[]): Promise<ReelThemePreset[]> {
        return webFetch('/admin/settings/reel-theme-presets', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(presets),
        });
    },

    // === HOME LAYOUT CONFIG ===

    async getHomeLayout(): Promise<HomeLayoutConfig> {
        return webFetch('/settings/home-layout');
    },

    async saveHomeLayout(config: HomeLayoutConfig): Promise<HomeLayoutConfig> {
        return webFetch('/admin/settings/home-layout', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(config),
        });
    },

    async getHomeLayoutPresets(): Promise<HomeLayoutPreset[]> {
        return webFetch('/admin/settings/home-layout-presets', { headers: authHeaders() });
    },

    async saveHomeLayoutPresets(presets: HomeLayoutPreset[]): Promise<HomeLayoutPreset[]> {
        return webFetch('/admin/settings/home-layout-presets', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(presets),
        });
    },

    // === COPY OVERRIDES ===

    async getCopyOverrides(): Promise<CopyOverrides> {
        return webFetch('/settings/copy');
    },

    async saveCopyOverrides(overrides: CopyOverrides): Promise<CopyOverrides> {
        return webFetch('/admin/settings/copy', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(overrides),
        });
    },

    // === DISPLAY CONFIG PRESETS ===

    async getDisplayPresets(): Promise<DisplayConfigPreset[]> {
        return webFetch('/admin/settings/display-presets', { headers: authHeaders() });
    },

    async saveDisplayPresets(presets: DisplayConfigPreset[]): Promise<DisplayConfigPreset[]> {
        return webFetch('/admin/settings/display-presets', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(presets),
        });
    },

    // === CABINET GAZETTE ===

    async getGazetteHome(loadFetch?: typeof fetch): Promise<GazetteHome> {
        try {
            return await webFetch('/gazette/home', undefined, loadFetch);
        } catch {
            return { leaves: [], cuttings: [] };
        }
    },

    async getGazetteRoom(year?: number, loadFetch?: typeof fetch): Promise<GazetteRoom> {
        try {
            const q = year != null ? `?year=${year}` : '';
            return await webFetch(`/gazette/blotter${q}`, undefined, loadFetch);
        } catch {
            return {
                year: year ?? new Date().getFullYear(),
                years: [],
                leaves: [],
                cuttings: [],
            };
        }
    },

    async getGazetteForWork(figurineId: string, loadFetch?: typeof fetch): Promise<GazetteLeaf[]> {
        try {
            return await webFetch(`/gazette/for-work/${encodeURIComponent(figurineId)}`, undefined, loadFetch);
        } catch {
            return [];
        }
    },

    /** The whole shelf of tall tales, in the order the keeper arranged it. */
    async getTales(loadFetch?: typeof fetch): Promise<GazetteLeaf[]> {
        try {
            return await webFetch('/tales', undefined, loadFetch);
        } catch {
            return [];
        }
    },

    /** Lay the shelf out top to bottom; position is the index in the array. */
    async adminReorderTales(ids: string[]): Promise<void> {
        await webFetch('/admin/gazette/tales/order', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ ids }),
        });
    },

    async getGazettePage(page = 1, perPage = 12, loadFetch?: typeof fetch): Promise<GazetteLeavesPage> {
        const q = new URLSearchParams({ page: String(page), perPage: String(perPage) });
        return webFetch(`/gazette?${q}`, undefined, loadFetch);
    },

    async getGazetteLeaf(slug: string, loadFetch?: typeof fetch): Promise<GazetteLeaf> {
        if (isGazetteReservedSlug(slug)) {
            throw new ApiError(404, 'not a leaf');
        }
        const leaf = await webFetch<GazetteLeaf>(`/gazette/${encodeURIComponent(slug)}`, undefined, loadFetch);
        if (typeof leaf?.titleEn !== 'string') {
            throw new ApiError(404, 'not a leaf');
        }
        return leaf;
    },

    async watchGazetteLeaf(
        slug: string,
        body: WatchGazetteLeafRequest,
        sessionToken?: string | null,
    ): Promise<GazetteWatchCreatedResponse> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
        return webFetch(`/gazette/${encodeURIComponent(slug)}/watch`, {
            method: 'POST',
            headers,
            body: JSON.stringify(body),
        });
    },

    async getGazetteWatchByToken(token: string): Promise<GazetteWatchInfo | null> {
        try {
            return await webFetch(`/gazette/watch/${encodeURIComponent(token)}`);
        } catch (err) {
            if (isNotFoundError(err)) return null;
            throw err;
        }
    },

    async leaveGazetteWatchByToken(token: string): Promise<void> {
        await webFetch(`/gazette/watch/${encodeURIComponent(token)}`, { method: 'POST' });
    },

    async userGazetteWatches(sessionToken: string): Promise<GazetteWatchDto[]> {
        return webFetch('/profile/gazette-watches', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async adminListGazetteLeaves(opts?: {
        status?: string;
        kind?: string;
        page?: number;
        perPage?: number;
    }): Promise<GazetteLeavesPage> {
        const q = new URLSearchParams();
        if (opts?.status) q.set('status', opts.status);
        if (opts?.kind) q.set('kind', opts.kind);
        if (opts?.page) q.set('page', String(opts.page));
        if (opts?.perPage) q.set('perPage', String(opts.perPage));
        const qs = q.toString();
        return webFetch(`/admin/gazette${qs ? `?${qs}` : ''}`, { headers: authHeaders() });
    },

    async adminGetGazetteLeaf(id: string): Promise<GazetteLeaf> {
        return webFetch(`/admin/gazette/${id}`, { headers: authHeaders() });
    },

    async adminSaveGazetteLeaf(body: SaveGazetteLeafRequest, id?: string): Promise<GazetteLeaf> {
        return webFetch(id ? `/admin/gazette/${id}` : '/admin/gazette', {
            method: id ? 'PUT' : 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    async adminDeleteGazetteLeaf(id: string): Promise<void> {
        await webFetch(`/admin/gazette/${id}`, { method: 'DELETE', headers: authHeaders() });
    },

    async adminListGazetteFeeds(): Promise<GazetteFeed[]> {
        return webFetch('/admin/gazette/feeds', { headers: authHeaders() });
    },

    async adminSaveGazetteFeed(body: SaveGazetteFeedRequest, id?: string): Promise<GazetteFeed> {
        return webFetch(id ? `/admin/gazette/feeds/${id}` : '/admin/gazette/feeds', {
            method: id ? 'PUT' : 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    async adminDeleteGazetteFeed(id: string): Promise<void> {
        await webFetch(`/admin/gazette/feeds/${id}`, { method: 'DELETE', headers: authHeaders() });
    },

    async adminRefreshGazetteDesk(): Promise<GazetteRefreshReport> {
        return webFetch('/admin/gazette/feeds/refresh', {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async adminListGazetteCuttings(opts?: {
        bucket?: 'inbox' | 'table' | 'aside' | 'all';
        feedId?: string;
        dismissed?: boolean;
        page?: number;
        perPage?: number;
    }): Promise<GazetteCuttingsPage> {
        const q = new URLSearchParams();
        if (opts?.bucket) q.set('bucket', opts.bucket);
        if (opts?.feedId) q.set('feedId', opts.feedId);
        if (opts?.dismissed) q.set('dismissed', 'true');
        if (opts?.page) q.set('page', String(opts.page));
        if (opts?.perPage) q.set('perPage', String(opts.perPage));
        const qs = q.toString();
        return webFetch(`/admin/gazette/cuttings${qs ? `?${qs}` : ''}`, { headers: authHeaders() });
    },

    async adminDismissGazetteCutting(id: string): Promise<void> {
        await webFetch(`/admin/gazette/cuttings/${id}/dismiss`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async adminRestoreGazetteCutting(id: string): Promise<void> {
        await webFetch(`/admin/gazette/cuttings/${id}/restore`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async adminPinGazetteCutting(id: string, pin: boolean): Promise<void> {
        await webFetch(`/admin/gazette/cuttings/${id}/${pin ? 'pin' : 'unpin'}`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async adminPromoteGazetteCutting(id: string): Promise<GazetteLeaf> {
        return webFetch(`/admin/gazette/cuttings/${id}/promote`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    // === СКРОМНЫЕ ЭПИЧЕСКИЕ БИТВЫ ===

    /** The whole shelf, in the order the keeper arranged it. Never paginated. */
    async getBattleCards(loadFetch?: typeof fetch): Promise<BattleCard[]> {
        try {
            return await webFetch('/battles/cards', undefined, loadFetch);
        } catch {
            // An empty room is a room with no cards in it yet, not a broken page.
            return [];
        }
    },

    async getBattleFrames(loadFetch?: typeof fetch): Promise<BattleFrames> {
        try {
            return await webFetch('/battles/frames', undefined, loadFetch);
        } catch {
            // The renderer falls back to its own five frames; see `frameFor`.
            return { frames: [] };
        }
    },

    /** The race dictionary. Same for every visitor, so cached like the shelf. */
    async getBattleRaces(loadFetch?: typeof fetch): Promise<BattleRace[]> {
        try {
            return await webFetch('/battles/races', undefined, loadFetch);
        } catch {
            return [];
        }
    },

    // ── Кошелёк и владение ───────────────────────────────────────────────
    //
    // Всё под именем. Книга лежит на сервере ровно потому, что кошелёк в
    // localStorage — это бесконечные деньги.

    /** Баланс и владение одним запросом: полка рисуется целиком или мигает. */
    async getBattleMe(sessionToken: string): Promise<BattleMe> {
        return webFetch('/battles/me', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    /** Стол гостя, с уже досчитанным заёмом: чем дом закрывает пустые места,
     *  решает сервер, а не страница. Второй реализации одного правила быть не
     *  должно — та, что в браузере, разошлась бы с той, по которой играют. */
    async getBattleDeck(sessionToken: string): Promise<BattleDeck> {
        return webFetch('/battles/deck', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    /** Разложить стол целиком. Половина сохранённой расстановки — не
     *  расстановка, а состояние, в котором её застали. */
    async saveBattleDeck(
        sessionToken: string,
        body: SaveBattleDeckRequest,
    ): Promise<BattleDeck> {
        return webFetch('/battles/deck', {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
                Authorization: `Bearer ${sessionToken}`,
            },
            body: JSON.stringify(body),
        });
    },

    /**
     * Взять карту с полки.
     *
     * `expectedPrice` — цена, которую видел человек. Сервер её не берёт, а
     * сверяет со своей: карта могла подорожать, пока страница стояла открытой.
     */
    async buyBattleCard(
        sessionToken: string,
        body: BuyBattleCardRequest,
    ): Promise<BuyBattleCardResponse> {
        return webFetch('/battles/buy', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify(body),
        });
    },

    /**
     * Поднять свой экземпляр на ступень.
     *
     * Ступень не называется: сервер читает уровень, который держит, и сам
     * решает, какая это ступень. Клиент, умеющий назвать ступень, умеет
     * назвать дешёвую.
     *
     * Уровень не трогает бой ни на очко (`TASKS-BATTLE-ENGINE.md` §1.6) —
     * это фольга и засечка, а не сила.
     */
    async raiseBattleCard(
        sessionToken: string,
        body: RaiseBattleCardRequest,
    ): Promise<RaiseBattleCardResponse> {
        return webFetch('/battles/raise', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify(body),
        });
    },

    /** Карту посмотрели — пометка «новая» снимается. */
    async markBattleCardSeen(sessionToken: string, cardId: string): Promise<void> {
        await webFetch(`/battles/cards/${cardId}/seen`, {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    /**
     * Внимание, за которое оседает пыль: сердечко, просмотренная работа,
     * прочитанная небылица. Однажды за каждую — ключ книги это и стережёт.
     *
     * Молчаливый: это маячок, а не просьба. Ошибку здесь показывать некому.
     */
    async grantBattleAttention(
        sessionToken: string,
        kind: 'seen' | 'read',
        id: string,
    ): Promise<BattleAttentionResponse | null> {
        try {
            return await webFetch('/battles/attention', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
                body: JSON.stringify({ kind, id }),
            });
        } catch {
            return null;
        }
    },

    async adminGetBattleDustRates(): Promise<BattleDustRates> {
        return webFetch('/admin/battles/dust-rates');
    },

    async adminSaveBattleDustRates(rates: BattleDustRates): Promise<BattleDustRates> {
        return webFetch('/admin/battles/dust-rates', {
            method: 'POST',
            body: JSON.stringify(rates),
        });
    },

    // ── Испытания и партии ───────────────────────────────────────────────
    //
    // Клиент не знает ни одного правила боя: он показывает состояние и
    // отправляет обратно одно из присланных законных действий.

    /**
     * The shelf of studies. Visible without a name; playing needs one.
     *
     * With a token the server also marks which studies this guest has already
     * been paid for — the reward belongs to the study, not to the victory.
     */
    async getBattleChallenges(
        sessionToken?: string | null,
        loadFetch?: typeof fetch,
    ): Promise<BattleChallenge[]> {
        try {
            const init = sessionToken
                ? { headers: { Authorization: `Bearer ${sessionToken}` } }
                : undefined;
            return await webFetch('/battles/challenges', init, loadFetch);
        } catch {
            return [];
        }
    },

    /** Begins a match — or continues the one this guest left going. */
    async beginBattleMatch(sessionToken: string, challengeId: string): Promise<BattleMatch> {
        return webFetch(`/battles/challenges/${challengeId}/begin`, {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async getBattleMatch(sessionToken: string, id: string): Promise<BattleMatch> {
        return webFetch(`/battles/matches/${id}`, {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    /**
     * One move, and the keeper's answer with it.
     *
     * `seq` is what makes a double click harmless: the server answers a repeat
     * with the same board instead of playing the move twice.
     */
    async actInBattleMatch(
        sessionToken: string,
        id: string,
        seq: number,
        action: BattleAction,
    ): Promise<BattleMatch> {
        return webFetch(`/battles/matches/${id}/act`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                Authorization: `Bearer ${sessionToken}`,
            },
            body: JSON.stringify({ seq, action }),
        });
    },

    /**
     * One step at the bench, or the whole thing played out.
     *
     * Writes nothing: the arrangement and the journal travel with the request.
     */
    async adminBenchBattle(body: BenchRequest): Promise<Bench> {
        return webFetch('/admin/battles/bench', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /** Сыгранные партии и сводка по ним. */
    async adminReadBattleMatches(): Promise<BattleMatches> {
        return webFetch('/admin/battles/matches', { headers: authHeaders() });
    },

    /** Пересмотр записанной партии до заданной ступени. */
    async adminReplayBattleMatch(id: string, upto: number): Promise<MatchReplay> {
        return webFetch(`/admin/battles/matches/${id}/replay?upto=${upto}`, {
            headers: authHeaders(),
        });
    },

    async adminListBattleChallenges(): Promise<BattleChallenge[]> {
        return webFetch('/admin/battles/challenges', { headers: authHeaders() });
    },

    async adminSaveBattleChallenge(
        body: SaveBattleChallengeRequest,
        id?: string,
    ): Promise<BattleChallenge> {
        return webFetch(id ? `/admin/battles/challenges/${id}` : '/admin/battles/challenges', {
            method: id ? 'PUT' : 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /** Порядок полки этюдов. Тот же приём, что и у полки карт: список целиком,
     *  а не «подвинь одно» — половина порядка порядком не является. */
    /** Что у гостя сейчас: монеты, карты, записки — ровно то, что видит он сам.
     *  Второго разбора нет намеренно: проверять игру надо по тому, что видит
     *  человек, а не по отчёту, который может с ним разойтись. */
    async adminReadBattleGuest(userId: string): Promise<BattleMe> {
        return webFetch(`/admin/battles/guest/${userId}`, { headers: authHeaders() });
    },

    /** Выдать карты гостю напрямую, минуя покупку. Кошелька не касается. */
    async adminGiveBattleCards(
        body: GiveBattleCardsRequest,
    ): Promise<GiveBattleCardsResponse> {
        return webFetch('/admin/battles/cards/give', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /** Забрать карты обратно — единственный способ проверить пустое собрание
     *  и временные карты, которые его закрывают. */
    async adminRevokeBattleCards(
        body: RevokeBattleCardsRequest,
    ): Promise<GiveBattleCardsResponse> {
        return webFetch('/admin/battles/cards/revoke', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /** Из рук хранителя — одному гостю, за настоящее. Единственный способ, каким
     *  в доме появляется корм: он не оседает сам, как пыль. */
    async adminGrantBattleCoin(
        body: GrantBattleCoinRequest,
    ): Promise<GrantBattleCoinResponse> {
        return webFetch('/admin/battles/grant', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    async adminReorderBattleChallenges(ids: string[]): Promise<void> {
        await webFetch('/admin/battles/challenges/reorder', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ ids }),
        });
    },

    async adminDeleteBattleChallenge(id: string): Promise<void> {
        await webFetch(`/admin/battles/challenges/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    /**
     * The scales, on a card that has not been saved.
     *
     * Called as the keeper types. The formula lives on the server and only
     * there — a second copy in the browser would drift, and the drift would be
     * noticed by a player rather than by a test.
     */
    async adminWeighBattleCard(body: SaveBattleCardRequest): Promise<BattleWeigh> {
        return webFetch('/admin/battles/weigh', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /**
     * The keyword dictionary. Public like the races: a reader who meets "Шипы 3"
     * on a card should be able to find out what Шипы means without an account.
     */
    async getBattleKeywords(loadFetch?: typeof fetch): Promise<BattleKeyword[]> {
        try {
            return await webFetch('/battles/keywords', undefined, loadFetch);
        } catch {
            return [];
        }
    },

    async adminSaveBattleKeyword(
        body: SaveBattleKeywordRequest,
        id?: string,
    ): Promise<BattleKeyword> {
        return webFetch(id ? `/admin/battles/keywords/${id}` : '/admin/battles/keywords', {
            method: id ? 'PUT' : 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /** Removes the wording, never the cards that named it. */
    async adminDeleteBattleKeyword(id: string): Promise<void> {
        await webFetch(`/admin/battles/keywords/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async adminSaveBattleRace(body: SaveBattleRaceRequest, id?: string): Promise<BattleRace> {
        return webFetch(id ? `/admin/battles/races/${id}` : '/admin/battles/races', {
            method: id ? 'PUT' : 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    /** Removes the dictionary entry, never the cards that wore it. */
    async adminDeleteBattleRace(id: string): Promise<void> {
        await webFetch(`/admin/battles/races/${id}`, { method: 'DELETE', headers: authHeaders() });
    },

    async adminListBattleCards(): Promise<BattleCard[]> {
        return webFetch('/admin/battles/cards', { headers: authHeaders() });
    },

    async adminSaveBattleCard(body: SaveBattleCardRequest, id?: string): Promise<BattleCard> {
        return webFetch(id ? `/admin/battles/cards/${id}` : '/admin/battles/cards', {
            method: id ? 'PUT' : 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(body),
        });
    },

    async adminDeleteBattleCard(id: string): Promise<void> {
        await webFetch(`/admin/battles/cards/${id}`, { method: 'DELETE', headers: authHeaders() });
    },

    /** Rewrite the whole shelf; position is the index in `ids`. */
    async adminReorderBattleCards(ids: string[]): Promise<void> {
        await webFetch('/admin/battles/cards/order', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ ids }),
        });
    },

    /**
     * Upload one card-frame picture, transparency intact.
     *
     * Not `importMediaWithVariants`: that writes four JPEG renditions, and a
     * JPEG of a cut-out frame is a rectangle of white where the card should
     * show through. Returns the picture's own size so the desk can set the
     * card's ratio from it, and whether it has a hole in it at all.
     */
    async adminUploadBattleFrameArt(
        file: File,
    ): Promise<{ url: string; width: number; height: number; hasAlpha: boolean }> {
        const form = new FormData();
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/admin/battles/frames/art`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
        const data = await res.json();
        return {
            url: webPublicUrl(data.url) ?? '',
            width: Number(data.width) || 0,
            height: Number(data.height) || 0,
            hasAlpha: !!data.hasAlpha,
        };
    },

    async adminSaveBattleFrames(config: BattleFrames): Promise<BattleFrames> {
        return webFetch('/admin/battles/frames', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(config),
        });
    },

    /**
     * Re-encode one image into another common format, at full resolution —
     * a one-off conversion for the keeper's own use, nothing saved on this
     * end. The only admin call that reads back a `Blob` instead of JSON: the
     * server hands back the converted file itself, not a URL to it.
     */
    async adminConvertImage(
        file: File,
        format: 'jpeg' | 'png' | 'webp',
        maxDimension?: number,
    ): Promise<Blob> {
        const form = new FormData();
        // Must be appended before `file` — the server reads fields in
        // arrival order (see admin_convert_image in handlers.rs).
        form.append('format', format);
        if (maxDimension) form.append('maxDimension', String(maxDimension));
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/admin/tools/convert-image`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) {
            const text = await res.text().catch(() => '');
            let message = text;
            try {
                const parsed = JSON.parse(text);
                if (parsed?.error) message = parsed.error;
            } catch {
                // Not JSON — the raw text is still better than nothing.
            }
            throw new Error(message || `Conversion failed: ${res.status}`);
        }
        return res.blob();
    },
};
