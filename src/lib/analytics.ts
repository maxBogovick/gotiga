import { lang } from '$lib/i18n';
import { get } from 'svelte/store';
import type { AnalyticsEventPayload } from '$lib/types/api';
import { sceneVariantForPath } from '$lib/sceneAb';

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

/** The author's own browser. Set when the admin signs in and never cleared by
 * itself: on a house this size the owner's own visits are a large share of
 * the traffic, and the server cannot tell them apart — a beacon carries no
 * Authorization header. */
const OWNER_KEY = 'gotiga_owner';
/** The first day this browser was seen (`YYYY-MM-DD`). A date many browsers
 * share, not an identifier: it only answers "earlier day or today". */
const FIRST_SEEN_KEY = 'gotiga_first_seen';

export function isOwnerBrowser(): boolean {
    try {
        return localStorage.getItem(OWNER_KEY) === '1';
    } catch {
        return false;
    }
}

export function setOwnerBrowser(on: boolean): void {
    try {
        if (on) localStorage.setItem(OWNER_KEY, '1');
        else localStorage.removeItem(OWNER_KEY);
    } catch {
        // Storage closed: the flag simply does not stick.
    }
}

function canTrack(): boolean {
    if (typeof window === 'undefined') return false;
    if (navigator.doNotTrack === '1') return false;
    if (location.pathname.startsWith('/admin')) return false;
    // Internal work surfaces (`/_frames-preview`) are not rooms of the house.
    if (location.pathname.startsWith('/_')) return false;
    if (isOwnerBrowser()) return false;
    return true;
}

let returningCache: boolean | null | undefined;

/** Whether this browser was here on an earlier day. `null` when storage is
 * closed — unknown is not "new", and the report keeps the two apart. Asked
 * only after `canTrack()` agreed: a visitor who sent Do Not Track gets nothing
 * written to their storage either. */
function cameBefore(): boolean | null {
    if (returningCache !== undefined) return returningCache;
    if (!canTrack()) return null;
    try {
        const today = new Date().toISOString().slice(0, 10);
        const first = localStorage.getItem(FIRST_SEEN_KEY);
        if (!first || !/^\d{4}-\d{2}-\d{2}$/.test(first)) {
            localStorage.setItem(FIRST_SEEN_KEY, today);
            returningCache = false;
        } else {
            returningCache = first < today;
        }
    } catch {
        returningCache = null;
    }
    return returningCache;
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

/** A path with its secret segment named, not spelled. */
export function scrubPath(path: string): string {
    const secret = SECRET_SEGMENT.find((pre) => path.startsWith(pre) && path.length > pre.length);
    return secret ? `${secret}:token` : path;
}

/** The address as analytics records it: the real path, minus anything that
 * would let the reader of the table act as the visitor. */
function trackedPath(): string {
    const path = scrubPath(location.pathname);
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
    | 'figurineId'
    | 'path'
    | 'referrer'
    | 'utmSource'
    | 'utmMedium'
    | 'utmCampaign'
    | 'pageViewId'
    | 'clientTs'
    | 'lang'
    | 'internalSource'
    | 'variant'
    | 'isReturning'
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
        // Which arm of a running experiment this page showed (`scene:on`).
        // Its own field: `src` and `utm_*` say where the visitor came from.
        variant: sceneVariantForPath(location.pathname) ?? null,
        isReturning: cameBefore(),
    };
}

const BEACON_URL = '/api/v1/analytics/events';

/** `allowed` is decided by the current address by default. The engagement
 * flush passes the answer it got when the room opened instead: it fires after
 * the visitor has already left, and by then `location` may point at /admin —
 * the page just left would lose its time and scroll to a rule written about
 * the page arrived at. */
function send(payload: AnalyticsEventPayload, allowed = canTrack()) {
    if (!allowed) return;
    const body = JSON.stringify(payload);
    const blob = new Blob([body], { type: 'text/plain;charset=UTF-8' });
    if (navigator.sendBeacon?.(BEACON_URL, blob)) return;
    // No `api` import here on purpose: `api` reports its own failures through
    // this module, and a cycle between the two would load one half-built.
    void fetch(BEACON_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'text/plain;charset=UTF-8' },
        body,
        keepalive: true,
    }).catch(() => undefined);
}

// ── Navigation, as the layout reports it ─────────────────────────────────

/** The room the visitor was in before this one (internal navigation only; a
 * first load has a `document.referrer` instead). */
let cameFrom: string | null = null;
/** Where an internal navigation in progress is heading. */
let goingTo: string | null = null;

/** Called by `+layout.svelte` from `beforeNavigate`. Held here, not read from
 * `location`, because by the time a room is closed `location` already points
 * at the destination. */
export function noteNavigation(from: URL | null | undefined, to: URL | null | undefined): void {
    if (typeof window === 'undefined') return;
    cameFrom = from && from.origin === location.origin ? scrubPath(from.pathname) : null;
    goingTo = to && to.origin === location.origin ? scrubPath(to.pathname) : null;
}

// ── Time the visitor was actually there ──────────────────────────────────

/** After this long without a touch, a scroll or a key, the visitor is taken to
 * have stepped away and the clock stops. Two minutes: long enough that a slow
 * reader of a long tale is not cut off between two scrolls, short enough that
 * a tab forgotten in the foreground does not count as a visit of an hour. */
const IDLE_MS = 120_000;

/** Foreground time: counts only while the tab is visible and the visitor is
 * not idle. Wall-clock time since the page opened — what the first version
 * sent — counted a tab opened in the background, a long pause to answer a
 * message, and a laptop closed overnight. */
function createDwell() {
    let total = 0;
    let segStart = 0;
    let last = 0;
    let on = false;

    const visible = () => document.visibilityState === 'visible';
    const close = (end: number) => {
        if (segStart) total += Math.max(0, end - segStart);
        segStart = 0;
    };
    const touch = () => {
        const t = Date.now();
        if (segStart && t - last > IDLE_MS) {
            // Stepped away and came back: the away stretch counts only up to
            // the idle limit.
            close(last + IDLE_MS);
            segStart = t;
        } else if (!segStart && visible()) {
            segStart = t;
        }
        last = t;
    };
    const onVisibility = () => {
        const t = Date.now();
        if (!visible()) {
            close(Math.min(t, last + IDLE_MS));
        } else if (!segStart) {
            // Only a stretch that is not already running starts one: a repeated
            // "visible" must not throw away the time counted so far.
            segStart = t;
            last = t;
        }
    };
    const EVENTS = ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart', 'scroll'] as const;

    return {
        start() {
            if (on) return;
            on = true;
            last = Date.now();
            segStart = visible() ? last : 0;
            for (const ev of EVENTS) window.addEventListener(ev, touch, { passive: true, capture: true });
            document.addEventListener('visibilitychange', onVisibility);
        },
        stop() {
            if (!on) return;
            on = false;
            for (const ev of EVENTS) window.removeEventListener(ev, touch, { capture: true });
            document.removeEventListener('visibilitychange', onVisibility);
        },
        /** Milliseconds of presence so far. */
        read(): number {
            const t = Date.now();
            const open = segStart ? Math.max(0, Math.min(t, last + IDLE_MS) - segStart) : 0;
            return Math.round(total + open);
        },
    };
}

/** Wall-clock delays between the progress snapshots of a visit that goes on:
 * the first catches the ones who stay ten seconds, the rest keep the row
 * fresh in case the page never gets to say goodbye (a phone that kills the
 * tab sends no `pagehide`). The last delay repeats. */
const BEATS = [10_000, 20_000, 30_000, 60_000, 120_000];

/** Rooms with an account or a secret in their address: no heading text from
 * them is ever recorded, because a heading there can be a person's name. */
const NO_SECTION = /^\/(profile|bookings|login|register|confirm|set-password|cancel|certificate|unsubscribe|gazette\/watch)/;

/** The last heading the visitor's deepest scroll reached — where on the page
 * the visit ended. Text of the heading, or its `data-section` when one is set;
 * the visible ones only. */
function headingReached(maxY: number): string | null {
    const reach = maxY + window.innerHeight;
    const root = document.querySelector('main') ?? document.body;
    let best: HTMLElement | null = null;
    let bestTop = -1;
    for (const el of root.querySelectorAll<HTMLElement>('h1, h2, h3, [data-section]')) {
        if (el.closest('nav, header, footer, dialog, [role="dialog"], [aria-hidden="true"]')) continue;
        const rect = el.getBoundingClientRect();
        if (rect.width === 0 && rect.height === 0) continue;
        const top = rect.top + window.scrollY;
        if (top <= reach && top >= bestTop) {
            best = el;
            bestTop = top;
        }
    }
    if (!best) return null;
    const text = (best.dataset.section ?? best.textContent ?? '').replace(/\s+/g, ' ').trim();
    return text ? text.slice(0, 80) : null;
}

/** A visitor's doing that is not a call to action: a search, a filter, a form
 * left half-filled. Deduped per room by name and detail unless `once` is off,
 * and capped, so a loop in a page cannot flood the table. */
const MAX_INTERACTIONS = 40;

/** Site-wide tracking for one visit to one page with no single figurine
 * attached. Same pipeline (batching, daily visitor hash, DNT/bot filtering) as
 * `createFigurineAnalytics`.
 *
 * Beyond the one-shot `page_view`, this measures engagement — how long the
 * visitor was present, how far they scrolled, which heading they reached and
 * where they went next — through `page_engaged` snapshots. The server keeps
 * one row per visit and lets the larger numbers win, so snapshots can be sent
 * as often as is useful: while the visit goes on, when the tab is hidden, and
 * when the room is left.
 *
 * Instances are made by `enterRoom` below, once per room the visitor walks
 * into, and never by a page: a page that must remember to call analytics is a
 * page that will one day be added without calling it, and the hole shows up a
 * month later as an empty row in the report. */
function createSiteAnalytics(arrivedFrom: string | null) {
    const pageViewId = crypto.randomUUID();
    const sent = new Set<string>();
    const worksSeen = new Set<string>();
    const dwell = createDwell();
    // Whether this page had work tiles at all. Not a flag passed in by the
    // page: the tiles say so themselves by registering, and a number is only
    // reported by a page that has something to count — a zero from /author
    // would read as "nobody reached the works" on a page that has none.
    let hasTiles = false;
    let mountedAt = 0;
    let maxScroll = 0;
    let maxY = 0;
    let section: string | null = null;
    let interactions = 0;
    let observer: IntersectionObserver | null = null;
    let listening = false;
    let allowed = false;
    let startPath = '';
    let beat = 0;
    let beatTimer: ReturnType<typeof setTimeout> | null = null;
    let sectionTimer: ReturnType<typeof setTimeout> | null = null;
    let outbound: string | null = null;
    let lastSnapshot = '';
    // Snapshot of the page's identity (path/referrer/utm/lang) taken at mount.
    // The final snapshot fires on teardown, and on a SvelteKit client-side
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

    /** True while the page on screen is still the one this room belongs to. */
    const onThisPage = () => location.pathname === startPath;

    function refreshSection() {
        if (NO_SECTION.test(startPath) || !onThisPage()) return;
        section = headingReached(maxY) ?? section;
    }

    function onScroll() {
        if (!onThisPage()) return;
        const depth = currentScrollDepth();
        if (depth > maxScroll) maxScroll = depth;
        if (window.scrollY > maxY) maxY = window.scrollY;
        if (sectionTimer) clearTimeout(sectionTimer);
        sectionTimer = setTimeout(refreshSection, 400);
    }

    function snapshot(exit: string | null) {
        if (!mountedAt || !engagedBase) return;
        if (onThisPage()) {
            onScroll();
            refreshSection();
        }
        const durationMs = dwell.read();
        const worksCount = hasTiles ? worksSeen.size : null;
        const fingerprint = `${durationMs}|${maxScroll}|${worksCount}|${exit}|${section}`;
        if (fingerprint === lastSnapshot) return;
        lastSnapshot = fingerprint;
        send(
            {
                ...engagedBase,
                eventType: 'page_engaged',
                durationMs,
                scrollDepth: maxScroll,
                worksSeen: worksCount,
                exitTo: exit,
                lastSection: section,
            },
            allowed,
        );
    }

    function armBeat() {
        if (beatTimer) clearTimeout(beatTimer);
        beatTimer = null;
        if (!listening || document.visibilityState !== 'visible') return;
        const wait = BEATS[Math.min(beat, BEATS.length - 1)];
        beatTimer = setTimeout(() => {
            beat++;
            // Progress only: where the visitor goes next is not known yet, and
            // a snapshot without it must not erase one that already has it.
            snapshot(null);
            armBeat();
        }, wait);
    }

    function handleVisibility() {
        if (document.visibilityState === 'hidden') {
            snapshot(outbound ?? 'hide');
            if (beatTimer) clearTimeout(beatTimer);
            beatTimer = null;
        } else {
            armBeat();
        }
    }

    function handlePageHide() {
        snapshot(outbound ?? 'hide');
    }

    /** A click on a link to another site is the exit; it is remembered because
     * the page unloads without telling anyone where it was going. */
    function handleClick(e: MouseEvent) {
        const a = (e.target as Element | null)?.closest?.('a[href]') as HTMLAnchorElement | null;
        if (!a || !a.hostname || a.origin === location.origin) return;
        if (a.protocol !== 'http:' && a.protocol !== 'https:') return;
        outbound = `ext:${a.hostname.toLowerCase()}`;
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
                // The room the visitor came from, when they came from one.
                detail: arrivedFrom,
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
        interact(name: string, detail?: string | null, value?: number | null, once = true) {
            const key = `i:${name}|${detail ?? ''}`;
            if (once && sent.has(key)) return;
            sent.add(key);
            if (++interactions > MAX_INTERACTIONS) return;
            send({
                ...basePayload(null, pageViewId),
                eventType: 'interaction',
                name,
                detail: detail ?? null,
                value: value ?? null,
            });
        },
        /** Foreground seconds so far — for an act that wants to say how long
         * the visitor took to reach it (a tale read to the end). */
        activeSeconds(): number {
            return Math.round(dwell.read() / 1000);
        },
        /** Begin dwell/scroll tracking, and pick up whatever work tiles are
         * already standing (see `tiles`). */
        start() {
            if (!canTrack() || listening) return;
            listening = true;
            allowed = true;
            mountedAt = Date.now();
            startPath = location.pathname;
            engagedBase = basePayload(null, pageViewId);
            dwell.start();
            onScroll();
            tiles.forEach(watch);
            window.addEventListener('scroll', onScroll, { passive: true });
            document.addEventListener('visibilitychange', handleVisibility);
            document.addEventListener('click', handleClick, true);
            window.addEventListener('pagehide', handlePageHide);
            armBeat();
        },
        watch,
        unwatch(node: HTMLElement) {
            observer?.unobserve(node);
        },
        /** Send the final snapshot, saying where the visitor went, and detach.
         * `next` is the room being entered; absent, the visitor left some other
         * way (closed the tab, followed a link out). */
        stop(next?: string | null) {
            if (listening) snapshot(next ?? outbound ?? 'hide');
            if (typeof window !== 'undefined') {
                window.removeEventListener('scroll', onScroll);
                document.removeEventListener('visibilitychange', handleVisibility);
                document.removeEventListener('click', handleClick, true);
                window.removeEventListener('pagehide', handlePageHide);
            }
            dwell.stop();
            if (beatTimer) clearTimeout(beatTimer);
            if (sectionTimer) clearTimeout(sectionTimer);
            observer?.disconnect();
            observer = null;
            listening = false;
        },
    };
}

type SiteAnalytics = ReturnType<typeof createSiteAnalytics>;

/** The figurine page: it reports itself (`figurine_view`), and its engagement
 * is the one the Works table is built on, so the contract is kept — a
 * `figurine_engaged` row means the visitor was present for eight seconds or
 * more. What changed is what the row says: the time is foreground time rather
 * than a timer that always fired at eight, the scroll is the deepest reached
 * rather than the position at that moment, and the row keeps growing while the
 * visit does. */
const FIGURINE_ENGAGED_MS = 8000;
/** ...or the visitor scrolled halfway down the page, whichever came first —
 * the rule the Works table was built on before the clock was fixed. */
const FIGURINE_ENGAGED_SCROLL = 50;

export function createFigurineAnalytics(figurineId: string) {
    const pageViewId = crypto.randomUUID();
    const sent = new Set<string>();
    const dwell = createDwell();
    let base: ReturnType<typeof basePayload> | null = null;
    let allowed = false;
    let started = false;
    let maxScroll = 0;
    let interactions = 0;
    let beat = 0;
    let beatTimer: ReturnType<typeof setTimeout> | null = null;
    let outbound: string | null = null;
    let lastSnapshot = '';

    /** Deepest scroll so far, as a percentage. */
    function measure() {
        const doc = document.documentElement;
        const scrollable = Math.max(1, doc.scrollHeight - window.innerHeight);
        const depth = Math.min(100, Math.max(0, Math.round((window.scrollY / scrollable) * 100)));
        if (depth > maxScroll) maxScroll = depth;
    }

    function onScroll() {
        const was = maxScroll;
        measure();
        // Halfway down is engagement on its own: say so now rather than
        // waiting for the next beat.
        if (started && was < FIGURINE_ENGAGED_SCROLL && maxScroll >= FIGURINE_ENGAGED_SCROLL) snapshot(null);
    }

    function snapshot(exit: string | null) {
        if (!started || !base) return;
        measure();
        const durationMs = dwell.read();
        // Neither eight seconds nor halfway down is a view that did not stay:
        // the Works table counts it as a view and not as an engaged one.
        if (durationMs < FIGURINE_ENGAGED_MS && maxScroll < FIGURINE_ENGAGED_SCROLL) return;
        const fingerprint = `${durationMs}|${maxScroll}|${exit}`;
        if (fingerprint === lastSnapshot) return;
        lastSnapshot = fingerprint;
        send(
            {
                ...base,
                eventType: 'figurine_engaged',
                durationMs,
                scrollDepth: maxScroll,
                exitTo: exit,
            },
            allowed,
        );
    }

    function armBeat() {
        if (beatTimer) clearTimeout(beatTimer);
        beatTimer = null;
        if (!started || document.visibilityState !== 'visible') return;
        // First look at eight seconds; then on the same slow schedule as rooms.
        const wait = beat === 0 ? FIGURINE_ENGAGED_MS : BEATS[Math.min(beat, BEATS.length - 1)];
        beatTimer = setTimeout(() => {
            beat++;
            snapshot(null);
            armBeat();
        }, wait);
    }

    function handleVisibility() {
        if (document.visibilityState === 'hidden') {
            snapshot(outbound ?? 'hide');
            if (beatTimer) clearTimeout(beatTimer);
            beatTimer = null;
        } else {
            armBeat();
        }
    }
    const handlePageHide = () => snapshot(outbound ?? 'hide');
    function handleClick(e: MouseEvent) {
        const a = (e.target as Element | null)?.closest?.('a[href]') as HTMLAnchorElement | null;
        if (!a || !a.hostname || a.origin === location.origin) return;
        if (a.protocol !== 'http:' && a.protocol !== 'https:') return;
        outbound = `ext:${a.hostname.toLowerCase()}`;
    }

    const tracker = {
        pageViewId,
        view() {
            if (sent.has('view')) return;
            sent.add('view');
            if (!canTrack()) return;
            allowed = true;
            started = true;
            base = basePayload(figurineId, pageViewId);
            send({
                ...base,
                eventType: 'figurine_view',
                detail: cameFrom,
            });
            dwell.start();
            window.addEventListener('scroll', onScroll, { passive: true });
            document.addEventListener('visibilitychange', handleVisibility);
            document.addEventListener('click', handleClick, true);
            window.addEventListener('pagehide', handlePageHide);
            activeFigurine = tracker;
            armBeat();
            flushPendingActs();
        },
        cta(ctaType: CtaType) {
            send({
                ...basePayload(figurineId, pageViewId),
                eventType: 'figurine_cta_click',
                ctaType,
            });
        },
        interact(name: string, detail?: string | null, value?: number | null, once = true) {
            const key = `i:${name}|${detail ?? ''}`;
            if (once && sent.has(key)) return;
            sent.add(key);
            if (++interactions > MAX_INTERACTIONS) return;
            send({
                ...basePayload(figurineId, pageViewId),
                eventType: 'interaction',
                name,
                detail: detail ?? null,
                value: value ?? null,
            });
        },
        activeSeconds(): number {
            return Math.round(dwell.read() / 1000);
        },
        /** The visitor left this page: send the last snapshot and detach. */
        end() {
            if (started) snapshot(goingTo ?? outbound ?? 'hide');
            started = false;
            if (typeof window !== 'undefined') {
                window.removeEventListener('scroll', onScroll);
                document.removeEventListener('visibilitychange', handleVisibility);
                document.removeEventListener('click', handleClick, true);
                window.removeEventListener('pagehide', handlePageHide);
            }
            dwell.stop();
            if (beatTimer) clearTimeout(beatTimer);
            if (activeFigurine === tracker) activeFigurine = null;
        },
    };
    return tracker;
}

/** Routes that report themselves and must not be reported twice. The figurine
 * page sends `figurine_view` from `FigurineDetailView`, and the daily rollups
 * count `page_view` and `figurine_view` side by side — a second event from the
 * layout would count one visit as two. Route ids, not paths: a route is what
 * the layout actually knows, and a path can be reached by slug or by uuid. */
const OWN_TRACKING = new Set(['/figurines/[id]']);

/** The room the visitor is in now, and its path. One page, one instance. */
let room: SiteAnalytics | null = null;
let roomPath: string | null = null;
/** The figurine page, while one is open — it is the tracker `interact` talks
 * to there, since that page has no room. */
let activeFigurine: ReturnType<typeof createFigurineAnalytics> | null = null;

/** Work tiles standing on screen, registered by the `observeWork` action.
 * Module-level, not a field on the room: a tile mounts while the DOM is built,
 * and the room opens in the layout's effect afterwards — held here, the order
 * of the two stops mattering. */
const tiles = new Set<HTMLElement>();

/** Called by `+layout.svelte` on every navigation — the single place the house
 * records that someone walked into a room. Leaves the previous room first
 * (sending its last snapshot, with this room named as where the visitor went),
 * then opens this one.
 *
 * Keyed on the **path**, not the whole address: `/figurines?series=…` and
 * `/battles?card=…` rewrite their query string as the visitor filters, and
 * each rewrite would otherwise close and reopen the same room, printing a
 * second visit and cutting the first one's time in half. */
export function enterRoom(path: string, routeId: string | null): void {
    if (typeof window === 'undefined') return;
    if (path === roomPath) return;
    leaveRoom(scrubPath(path));
    roomPath = path;
    if (routeId && OWN_TRACKING.has(routeId)) return;
    room = createSiteAnalytics(cameFrom);
    room.pageView();
    room.start();
    flushPendingActs();
}

/** Close the current room (sends its last snapshot). Called on teardown; a
 * full unload is covered by the instance's own `pagehide` listener. */
export function leaveRoom(next?: string | null): void {
    room?.stop(next);
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

// ── Doings: search, filters, gallery, forms, reading ─────────────────────

/** Record something the visitor did on this page. `name` is a short word
 * (`search`, `filter`, `gallery_frame`), `detail` says what it was about, and
 * `value` how many or how much — results found, a frame number. Never put
 * anything the visitor typed into `detail` except a search term; the server
 * strips addresses, but it cannot know what a field was. */
export function interact(name: string, detail?: string | null, value?: number | null, once = true): void {
    if (typeof window === 'undefined' || !canTrack()) return;
    const target = activeFigurine ?? (roomPath === location.pathname ? room : null);
    if (target) {
        target.interact(name, detail, value, once);
        return;
    }
    // The page is up but its room is not yet: a page's own effects run before
    // the layout's, so on arrival the room still belongs to the page just left
    // (or does not exist at all on a first load). Held until the room opens.
    if (pendingActs.length < 20) pendingActs.push([name, detail, value, once]);
}

const pendingActs: [string, string | null | undefined, number | null | undefined, boolean][] = [];

function flushPendingActs() {
    const target = activeFigurine ?? room;
    if (!target) return;
    for (const [name, detail, value, once] of pendingActs.splice(0)) target.interact(name, detail, value, once);
}

/** Svelte action for a block of work tiles (each marked by `observeWork`): says
 * which tile — by its place in the block — a click on a work link came from, and
 * when the block's own "the rest is in the archive" link was the way out.
 * Delegated, so the tiles themselves know nothing about it. */
export function trackWorkClicks(node: HTMLElement, surface: string) {
    const onClick = (e: MouseEvent) => {
        const a = (e.target as Element | null)?.closest?.('a[href]') as HTMLAnchorElement | null;
        if (!a || a.origin !== location.origin) return;
        const tile = a.closest('[data-work-id]');
        if (tile && a.pathname.startsWith('/figurines/')) {
            const place = Array.from(node.querySelectorAll('[data-work-id]')).indexOf(tile) + 1;
            if (place > 0) interact('work_click', `${surface}:${place}`, place, false);
        } else if (a.pathname === '/figurines') {
            interact('archive_end', surface);
        }
    };
    node.addEventListener('click', onClick, true);
    return { destroy: () => node.removeEventListener('click', onClick, true) };
}

/** Foreground seconds on the current page, or null when none is being measured. */
export function activeSeconds(): number | null {
    if (typeof window === 'undefined') return null;
    const t = activeFigurine ?? room;
    return t ? t.activeSeconds() : null;
}

interface FormState {
    name: string;
    started: boolean;
    done: boolean;
    last: string;
    /** Outcome names that count as this form having been sent. One request
     * form can end as an order, a booking, a waitlist place or a commission. */
    accepts: Set<string>;
}
/** Every form on screen. A set, not a map by name: the footer and the page can
 * each carry a newsletter form at the same time. */
const forms = new Set<FormState>();

/** What the visitor was in when they stopped — the field's name, never its value. */
function fieldName(target: EventTarget | null): string {
    const el = target as HTMLElement | null;
    const raw =
        el?.getAttribute?.('name') ||
        el?.id ||
        el?.getAttribute?.('aria-label') ||
        el?.tagName?.toLowerCase() ||
        'field';
    return raw.replace(/[^\w.-]/g, '').slice(0, 30) || 'field';
}

/** Svelte action for a `<form>`: notes when someone starts filling it in, which
 * field a native check refused, and — the point of it — which field they were
 * in when they left without sending. Success and failure are reported by
 * `formOutcome`, called where the answer is known (the API layer). */
export function trackForm(node: HTMLElement, param: string | { name: string; accepts?: string[] }) {
    const name = typeof param === 'string' ? param : param.name;
    const accepts = new Set(typeof param === 'string' ? [param] : [param.name, ...(param.accepts ?? [])]);
    const state: FormState = { name, started: false, done: false, last: '', accepts };
    forms.add(state);

    const onFocus = (e: Event) => {
        const t = e.target;
        if (!(t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || t instanceof HTMLSelectElement)) return;
        state.last = fieldName(t);
        if (!state.started) {
            state.started = true;
            state.done = false;
            interact('form_start', name);
        }
    };
    const onInvalid = (e: Event) => interact('form_error', `${name}:${fieldName(e.target)}`);
    const abandon = () => {
        if (state.started && !state.done) interact('form_abandon', `${name}:${state.last || 'none'}`);
        state.started = false;
    };

    node.addEventListener('focusin', onFocus);
    node.addEventListener('invalid', onInvalid, true);
    window.addEventListener('pagehide', abandon);
    return {
        destroy() {
            node.removeEventListener('focusin', onFocus);
            node.removeEventListener('invalid', onInvalid, true);
            window.removeEventListener('pagehide', abandon);
            abandon();
            forms.delete(state);
        },
    };
}

/** The answer to a form: it was accepted, or it was not (`why` is a status or
 * a short word, not text from the server). Called from `api`, once per call,
 * so no form has to remember to report its own result. */
export function formOutcome(name: string, ok: boolean, why?: string | number): void {
    if (ok) {
        for (const state of forms) {
            if (state.accepts.has(name)) state.done = true;
        }
    }
    interact(ok ? 'form_ok' : 'form_fail', why !== undefined ? `${name}:${why}` : name, undefined, false);
}

// ── Breakage ─────────────────────────────────────────────────────────────

const errorsSeen = new Set<string>();
const MAX_ERRORS_PER_LOAD = 8;

/** Strip what must not be recorded from a piece of text that came from the
 * page: query strings and fragments off every address in it. */
function scrubText(text: string): string {
    return text
        .replace(/(https?:\/\/[^\s?#"')]+)[?#][^\s"')]*/g, '$1')
        .replace(/\s+/g, ' ')
        .trim()
        .slice(0, 160);
}

/** Tell the house something broke on the visitor's side. `kind` is one of
 * `js`, `promise`, `image`, `api`, `resource`, `render`. Deduped by kind and
 * text and capped per page load: a broken loop in a page would otherwise turn
 * every visitor into a firehose. */
export function reportError(kind: 'js' | 'promise' | 'image' | 'api' | 'resource' | 'render', detail: string): void {
    if (typeof window === 'undefined' || !canTrack()) return;
    const text = scrubText(detail);
    if (!text) return;
    // A browser extension's own failure is not the house's.
    if (/(chrome|moz|safari-web)-extension:/.test(detail)) return;
    if (/ResizeObserver loop/i.test(text)) return;
    // A view transition cut short by the next navigation rejects its promise;
    // it is the browser doing what it was told, not a fault of the page.
    if (/Transition was (aborted|skipped)|AbortError/i.test(text)) return;
    const key = `${kind}|${text}`;
    if (errorsSeen.has(key) || errorsSeen.size >= MAX_ERRORS_PER_LOAD) return;
    errorsSeen.add(key);
    send({
        ...basePayload(null, crypto.randomUUID()),
        eventType: 'client_error',
        name: kind,
        detail: text,
    });
}

/** A request to the API failed in a way the visitor can feel (the network, or
 * a 5xx). The path is generalised — ids and tokens become `:id` — so the same
 * failing endpoint is one row, not one per work. */
export function reportApiFailure(method: string, path: string, status: number | 'network'): void {
    // Leaving the page cancels in-flight requests; that is not a failure.
    if (typeof document === 'undefined' || document.visibilityState === 'hidden') return;
    if (status === 'network' && typeof navigator !== 'undefined' && navigator.onLine === false) return;
    const generic = path
        .split('?')[0]
        .replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, ':id')
        .replace(/\/[A-Za-z0-9_-]{24,}/g, '/:token');
    if (generic.includes('/analytics/')) return;
    reportError('api', `${method} ${generic} ${status}`);
}

let errorsInstalled = false;

/** Listen for script errors, unhandled rejections and failed images. Once, from
 * the layout. The resource listener runs in the capture phase because an image
 * error does not bubble. */
export function installErrorReporting(): void {
    if (typeof window === 'undefined' || errorsInstalled) return;
    errorsInstalled = true;
    window.addEventListener(
        'error',
        (e: Event) => {
            const target = e.target as HTMLElement | null;
            if (target && target !== (window as unknown as HTMLElement) && target.tagName) {
                const tag = target.tagName.toLowerCase();
                if (tag === 'img' || tag === 'source') {
                    const src = (target as HTMLImageElement).currentSrc || (target as HTMLImageElement).src || '';
                    reportError('image', src.replace(location.origin, ''));
                } else if (tag === 'script' || tag === 'link') {
                    const url = (target as HTMLScriptElement).src || (target as HTMLLinkElement).href || '';
                    reportError('resource', url.replace(location.origin, ''));
                }
                return;
            }
            const err = e as ErrorEvent;
            const where = err.filename ? ` @ ${err.filename.split('/').pop()}:${err.lineno}` : '';
            // A cross-origin script reports only "Script error." with no
            // detail; it is still counted, but it says nothing more.
            reportError('js', `${err.message || 'error'}${where}`);
        },
        true,
    );
    window.addEventListener('unhandledrejection', (e) => {
        const reason = e.reason as { message?: string } | string | undefined;
        const text = typeof reason === 'string' ? reason : (reason?.message ?? String(reason));
        reportError('promise', text);
    });
}

/** A page that does not exist was asked for. `cameFrom` is the room the visitor
 * clicked in; for an outside arrival the event's referrer says it instead. */
export function reportNotFound(): void {
    if (typeof window === 'undefined' || !canTrack()) return;
    send({
        ...basePayload(null, crypto.randomUUID()),
        eventType: 'not_found',
        detail: cameFrom,
    });
}

// ── Speed, as the visitor's device saw it ────────────────────────────────

let vitalsStarted = false;

/** Largest contentful paint, interaction latency, layout shift and server
 * response time for the page the visitor landed on. One row per load,
 * re-sent (the server keeps the larger numbers) when the tab is hidden —
 * LCP and CLS keep changing until the visitor does something. A client-side
 * navigation does not reset them, so only the landing page is measured. */
export function startVitals(): void {
    if (typeof window === 'undefined' || vitalsStarted || !canTrack()) return;
    if (typeof PerformanceObserver === 'undefined') return;
    vitalsStarted = true;

    const id = crypto.randomUUID();
    const landing = trackedPath();
    let lcp: number | null = null;
    let cls = 0;
    let sessionValue = 0;
    let sessionFirst = 0;
    let sessionLast = 0;
    let inp: number | null = null;
    const longest: number[] = [];
    const seenInteractions = new Set<number>();

    const observe = (type: string, cb: (entries: PerformanceEntryList) => void, extra: object = {}) => {
        try {
            const po = new PerformanceObserver((list) => cb(list.getEntries()));
            po.observe({ type, buffered: true, ...extra } as PerformanceObserverInit);
        } catch {
            // The browser does not know this entry type.
        }
    };

    observe('largest-contentful-paint', (entries) => {
        const last = entries[entries.length - 1];
        if (last) lcp = last.startTime;
    });

    observe('layout-shift', (entries) => {
        for (const entry of entries as unknown as { value: number; hadRecentInput: boolean; startTime: number }[]) {
            if (entry.hadRecentInput) continue;
            // CLS is the worst "session window": shifts less than a second
            // apart, in a window no longer than five seconds.
            if (sessionValue && entry.startTime - sessionLast < 1000 && entry.startTime - sessionFirst < 5000) {
                sessionValue += entry.value;
            } else {
                sessionValue = entry.value;
                sessionFirst = entry.startTime;
            }
            sessionLast = entry.startTime;
            if (sessionValue > cls) cls = sessionValue;
        }
    });

    observe(
        'event',
        (entries) => {
            for (const entry of entries as unknown as { interactionId?: number; duration: number }[]) {
                if (!entry.interactionId || seenInteractions.has(entry.interactionId)) continue;
                seenInteractions.add(entry.interactionId);
                longest.push(entry.duration);
                longest.sort((a, b) => b - a);
                if (longest.length > 10) longest.length = 10;
                // INP is the 98th percentile: the worst interaction, or the
                // one below it for every fifty interactions after the first.
                inp = longest[Math.min(longest.length - 1, Math.floor(seenInteractions.size / 50))];
            }
        },
        { durationThreshold: 40 },
    );

    const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined;
    const ttfb = nav && nav.responseStart > 0 ? Math.round(nav.responseStart) : null;
    const connection = (navigator as unknown as { connection?: { effectiveType?: string } }).connection;

    let lastSent = '';
    const flush = () => {
        const payload = {
            vitalLcp: lcp === null ? null : Math.min(120_000, Math.round(lcp)),
            vitalInp: inp === null ? null : Math.min(60_000, Math.round(inp)),
            vitalCls: Math.min(10_000, Math.round(cls * 1000)),
            vitalTtfb: ttfb,
        };
        // Nothing painted yet and nothing to say: an empty row is not a sample.
        if (payload.vitalLcp === null && payload.vitalInp === null && !payload.vitalCls && payload.vitalTtfb === null) return;
        const fingerprint = JSON.stringify(payload);
        if (fingerprint === lastSent) return;
        lastSent = fingerprint;
        send({
            ...basePayload(null, id),
            path: landing,
            eventType: 'web_vitals',
            ...payload,
            conn: connection?.effectiveType ?? null,
            viewportW: Math.round(window.innerWidth),
        });
    };
    document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'hidden') flush();
    });
    window.addEventListener('pagehide', flush);
}
