import { api } from '$lib/api';
import { lang } from '$lib/i18n';
import { get } from 'svelte/store';
import type { AnalyticsEventPayload } from '$lib/types/api';

type CtaType =
    | 'request'
    | 'reserve'
    | 'booking'
    | 'waitlist'
    | 'notify'
    | 'create_similar'
    | 'wishlist'
    | 'comment'
    | 'passport'
    | 'related_figurine'
    | 'commission_form_start'
    | string;

function canTrack(): boolean {
    if (typeof window === 'undefined') return false;
    if (navigator.doNotTrack === '1') return false;
    if (location.pathname.startsWith('/admin')) return false;
    // Internal work surfaces (`/_frames-preview`) are not rooms of the house.
    if (location.pathname.startsWith('/_')) return false;
    return true;
}

/** Routes whose address itself is the key: a cancel token, a certificate
 * token, an unsubscribe token, a gazette watch token. Analytics keeps a path,
 * never a key — the secret segment is replaced before the event is built, and
 * replaced here rather than at each call site, because every event in this
 * module goes through `basePayload`. Written as prefixes, not as route ids:
 * `location` is all an event has. */
const SECRET_SEGMENT = ['/cancel/', '/certificate/', '/unsubscribe/', '/gazette/watch/'];

/** Query keys carrying the same kind of secret (`/confirm?token=…`). Dropped
 * whole: which link was opened is already said by the path. */
const SECRET_QUERY = new Set(['token', 'code', 'key', 'hash', 'email']);

/** The address as analytics records it: the real path, minus anything that
 * would let the reader of the table act as the visitor. */
function trackedPath(): string {
    let path = location.pathname;
    const secret = SECRET_SEGMENT.find((pre) => path.startsWith(pre) && path.length > pre.length);
    if (secret) path = `${secret}:token`;
    const q = new URLSearchParams(location.search);
    for (const key of [...q.keys()]) {
        if (SECRET_QUERY.has(key.toLowerCase())) q.delete(key);
    }
    const search = q.toString();
    return search ? `${path}?${search}` : path;
}

function utm(name: string): string | null {
    try {
        return new URL(location.href).searchParams.get(name);
    } catch {
        return null;
    }
}

function basePayload(figurineId: string | null, pageViewId: string): Pick<
    AnalyticsEventPayload,
    'figurineId' | 'path' | 'referrer' | 'utmSource' | 'utmMedium' | 'utmCampaign' | 'pageViewId' | 'clientTs' | 'lang' | 'internalSource'
> {
    return {
        figurineId: figurineId ?? undefined,
        path: trackedPath(),
        referrer: document.referrer || null,
        utmSource: utm('utm_source'),
        utmMedium: utm('utm_medium'),
        utmCampaign: utm('utm_campaign'),
        pageViewId,
        clientTs: new Date().toISOString(),
        lang: get(lang),
        // Which on-site block a figurine-card click came from (e.g.
        // "home_afisha"), tagged by the linking component via `?src=` — kept
        // separate from utm_source, which is for external campaigns.
        internalSource: utm('src'),
    };
}

/** `allowed` is decided by the current address by default. The engagement
 * flush passes the answer it got when the room opened instead: it fires after
 * the visitor has already left, and by then `location` may point at /admin —
 * the page just left would lose its time and scroll to a rule written about
 * the page arrived at. */
function send(payload: AnalyticsEventPayload, allowed = canTrack()) {
    if (!allowed) return;
    const body = JSON.stringify(payload);
    const blob = new Blob([body], { type: 'text/plain;charset=UTF-8' });
    const url = '/api/v1/analytics/events';
    if (navigator.sendBeacon?.(url, blob)) return;
    void api.sendAnalyticsEvent(payload);
}

export function createFigurineAnalytics(figurineId: string) {
    const pageViewId = crypto.randomUUID();
    const sent = new Set<string>();

    return {
        pageViewId,
        view() {
            if (sent.has('view')) return;
            sent.add('view');
            send({
                ...basePayload(figurineId, pageViewId),
                eventType: 'figurine_view',
            });
        },
        engaged(data?: { durationMs?: number; scrollDepth?: number }) {
            if (sent.has('engaged')) return;
            sent.add('engaged');
            send({
                ...basePayload(figurineId, pageViewId),
                eventType: 'figurine_engaged',
                durationMs: data?.durationMs ?? null,
                scrollDepth: data?.scrollDepth ?? null,
            });
        },
        cta(ctaType: CtaType) {
            send({
                ...basePayload(figurineId, pageViewId),
                eventType: 'figurine_cta_click',
                ctaType,
            });
        },
    };
}

/** Site-wide tracking for one visit to one page with no single figurine
 * attached. Same pipeline (batching, daily visitor hash, DNT/bot filtering) as
 * `createFigurineAnalytics`.
 *
 * Beyond the one-shot `page_view`, this measures engagement — how long the
 * visitor stayed and how far they scrolled — via a single `page_engaged` event
 * flushed when the page is backgrounded or left.
 *
 * Instances are made by `enterRoom` below, once per room the visitor walks
 * into, and never by a page: a page that must remember to call analytics is a
 * page that will one day be added without calling it, and the hole shows up a
 * month later as an empty row in the report. */
function createSiteAnalytics() {
    const pageViewId = crypto.randomUUID();
    const sent = new Set<string>();
    const worksSeen = new Set<string>();
    // Whether this page had work tiles at all. Not a flag passed in by the
    // page: the tiles say so themselves by registering, and a number is only
    // reported by a page that has something to count — a zero from /author
    // would read as "nobody reached the works" on a page that has none.
    let hasTiles = false;
    let mountedAt = 0;
    let maxScroll = 0;
    let observer: IntersectionObserver | null = null;
    let listening = false;
    let allowed = false;
    // Snapshot of the page's identity (path/referrer/utm/lang) taken at mount.
    // The engaged event fires on teardown, and on a SvelteKit client-side
    // navigation `location` has already advanced to the *destination* route by
    // the time it runs — reading it then would misattribute this page's
    // time/scroll/works to the next page. Captured here, it stays correct.
    let engagedBase: ReturnType<typeof basePayload> | null = null;

    function currentScrollDepth(): number {
        if (typeof window === 'undefined') return 0;
        const doc = document.documentElement;
        const scrollable = Math.max(1, doc.scrollHeight - window.innerHeight);
        return Math.min(100, Math.max(0, Math.round((window.scrollY / scrollable) * 100)));
    }

    function onScroll() {
        const depth = currentScrollDepth();
        if (depth > maxScroll) maxScroll = depth;
    }

    // The final engagement flush. Fired once — on tab-background
    // (visibilitychange→hidden, the reliable signal on mobile where pagehide is
    // flaky), on pagehide, or when the visitor leaves the room — reporting
    // foreground time and the deepest scroll reached.
    function flushEngaged() {
        if (sent.has('page_engaged') || !mountedAt || !engagedBase) return;
        sent.add('page_engaged');
        onScroll();
        send(
            {
                ...engagedBase,
                eventType: 'page_engaged',
                durationMs: Math.max(0, Date.now() - mountedAt),
                scrollDepth: maxScroll,
                worksSeen: hasTiles ? worksSeen.size : null,
            },
            allowed,
        );
    }

    function handleVisibility() {
        if (document.visibilityState === 'hidden') flushEngaged();
    }

    function ensureObserver(): IntersectionObserver | null {
        if (!canTrack()) return null;
        if (!observer) {
            // A tile counts as "seen" once it reaches the central band of the
            // viewport — not merely peeking in at the very edge. Expressed as a
            // rootMargin band rather than a visibility ratio on purpose: a
            // full-height reel pane taller than the viewport never reaches a
            // 50%-visible ratio, but it does cross this band.
            observer = new IntersectionObserver(
                (entries) => {
                    for (const e of entries) {
                        if (!e.isIntersecting) continue;
                        const id = (e.target as HTMLElement).dataset.workId;
                        if (id) worksSeen.add(id);
                    }
                },
                { rootMargin: '-25% 0px -25% 0px', threshold: 0 },
            );
        }
        return observer;
    }

    function watch(node: HTMLElement) {
        hasTiles = true;
        ensureObserver()?.observe(node);
    }

    return {
        pageViewId,
        pageView() {
            if (sent.has('page_view')) return;
            sent.add('page_view');
            send({
                ...basePayload(null, pageViewId),
                eventType: 'page_view',
            });
        },
        cta(ctaType: CtaType) {
            if (sent.has(`cta:${ctaType}`)) return;
            sent.add(`cta:${ctaType}`);
            send({
                ...basePayload(null, pageViewId),
                eventType: 'figurine_cta_click',
                ctaType,
            });
        },
        /** Begin dwell/scroll tracking, and pick up whatever work tiles are
         * already standing (see `tiles`). */
        start() {
            if (!canTrack() || listening) return;
            listening = true;
            allowed = true;
            mountedAt = Date.now();
            engagedBase = basePayload(null, pageViewId);
            onScroll();
            tiles.forEach(watch);
            window.addEventListener('scroll', onScroll, { passive: true });
            document.addEventListener('visibilitychange', handleVisibility);
            window.addEventListener('pagehide', flushEngaged);
        },
        watch,
        unwatch(node: HTMLElement) {
            observer?.unobserve(node);
        },
        /** Flush the final `page_engaged` event and detach listeners. */
        stop() {
            flushEngaged();
            if (typeof window !== 'undefined') {
                window.removeEventListener('scroll', onScroll);
                document.removeEventListener('visibilitychange', handleVisibility);
                window.removeEventListener('pagehide', flushEngaged);
            }
            observer?.disconnect();
            observer = null;
            listening = false;
        },
    };
}

type SiteAnalytics = ReturnType<typeof createSiteAnalytics>;

/** Routes that report themselves and must not be reported twice. The figurine
 * page sends `figurine_view` from `FigurineDetailView`, and the daily rollups
 * count `page_view` and `figurine_view` side by side — a second event from the
 * layout would count one visit as two. Route ids, not paths: a route is what
 * the layout actually knows, and a path can be reached by slug or by uuid. */
const OWN_TRACKING = new Set(['/figurines/[id]']);

/** The room the visitor is in now, and its path. One page, one instance. */
let room: SiteAnalytics | null = null;
let roomPath: string | null = null;

/** Work tiles standing on screen, registered by the `observeWork` action.
 * Module-level, not a field on the room: a tile mounts while the DOM is built,
 * and the room opens in the layout's effect afterwards — held here, the order
 * of the two stops mattering. */
const tiles = new Set<HTMLElement>();

/** Called by `+layout.svelte` on every navigation — the single place the house
 * records that someone walked into a room. Leaves the previous room first
 * (flushing its dwell time), then opens this one.
 *
 * Keyed on the **path**, not the whole address: `/figurines?series=…` and
 * `/battles?card=…` rewrite their query string as the visitor filters, and
 * each rewrite would otherwise close and reopen the same room, printing a
 * second visit and cutting the first one's time in half. */
export function enterRoom(path: string, routeId: string | null): void {
    if (typeof window === 'undefined') return;
    if (path === roomPath) return;
    leaveRoom();
    roomPath = path;
    if (routeId && OWN_TRACKING.has(routeId)) return;
    room = createSiteAnalytics();
    room.pageView();
    room.start();
}

/** Close the current room (flushes `page_engaged`). Called on teardown; a full
 * unload is covered by the instance's own `pagehide` listener. */
export function leaveRoom(): void {
    room?.stop();
    room = null;
    roomPath = null;
}

/** Svelte action for a work tile on a grid — counts the tile toward this
 * visit's `works_seen` once it scrolls into view. */
export function observeWork(node: HTMLElement, id: string) {
    node.dataset.workId = id;
    tiles.add(node);
    room?.watch(node);
    return {
        destroy() {
            tiles.delete(node);
            room?.unwatch(node);
        },
    };
}

/** A deliberate act inside the current room that is not about one figurine —
 * opening the commission form, saving a work from the reel. Deduped per room,
 * so calling it on every click of the same button still records one. */
export function roomCta(ctaType: CtaType): void {
    room?.cta(ctaType);
}
