import { api, isTauri } from '$lib/api';
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
    if (typeof window === 'undefined' || isTauri) return false;
    if (navigator.doNotTrack === '1') return false;
    if (location.pathname.startsWith('/admin')) return false;
    return true;
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
        path: `${location.pathname}${location.search}`,
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

function send(payload: AnalyticsEventPayload) {
    if (!canTrack()) return;
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

/** Site-wide tracking for pages with no single figurine — home, archive,
 * /author, /workshop, /commission. Same pipeline (batching, daily visitor
 * hash, DNT/bot filtering) as `createFigurineAnalytics`, just without a
 * figurine attached.
 *
 * Beyond the one-shot `page_view`, this also measures engagement — how long the
 * visitor stayed and how far they scrolled — via a single `page_engaged` event
 * flushed when the page is backgrounded or left. Pass `{ trackWorks: true }` on
 * the home/archive grids to also count how many distinct work tiles the visitor
 * actually saw (attach `observeWork` to each tile). */
export function createSiteAnalytics(opts?: { trackWorks?: boolean }) {
    const pageViewId = crypto.randomUUID();
    const sent = new Set<string>();
    const trackWorks = opts?.trackWorks ?? false;
    const worksSeen = new Set<string>();
    let mountedAt = 0;
    let maxScroll = 0;
    let observer: IntersectionObserver | null = null;
    let listening = false;
    // Snapshot of the page's identity (path/referrer/utm/lang) taken at mount.
    // The engaged event fires on teardown, and on a SvelteKit client-side
    // navigation `location` has already advanced to the *destination* route by
    // the time onDestroy runs — reading it then would misattribute this page's
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
    // flaky), on pagehide, or on component teardown — reporting foreground time
    // and the deepest scroll reached.
    function flushEngaged() {
        if (sent.has('page_engaged') || !mountedAt || !engagedBase) return;
        sent.add('page_engaged');
        onScroll();
        send({
            ...engagedBase,
            eventType: 'page_engaged',
            durationMs: Math.max(0, Date.now() - mountedAt),
            scrollDepth: maxScroll,
            worksSeen: trackWorks ? worksSeen.size : null,
        });
    }

    function handleVisibility() {
        if (document.visibilityState === 'hidden') flushEngaged();
    }

    function ensureObserver(): IntersectionObserver | null {
        if (!trackWorks || !canTrack()) return null;
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
        /** Begin dwell/scroll tracking. Call once in `onMount` (browser only). */
        start() {
            if (!canTrack() || listening) return;
            listening = true;
            mountedAt = Date.now();
            engagedBase = basePayload(null, pageViewId);
            onScroll();
            window.addEventListener('scroll', onScroll, { passive: true });
            document.addEventListener('visibilitychange', handleVisibility);
            window.addEventListener('pagehide', flushEngaged);
        },
        /** Svelte action for work tiles on the home/archive grids — records the
         * tile toward `works_seen` once it scrolls into view. No-op unless the
         * instance was created with `{ trackWorks: true }`. */
        observeWork(node: HTMLElement, id: string) {
            node.dataset.workId = id;
            ensureObserver()?.observe(node);
            return {
                destroy() {
                    observer?.unobserve(node);
                },
            };
        },
        /** Flush the final `page_engaged` event and detach listeners. Call in
         * `onDestroy` (covers SPA navigation; pagehide covers full unload). */
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
