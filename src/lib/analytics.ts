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
 * figurine attached. */
export function createSiteAnalytics() {
    const pageViewId = crypto.randomUUID();
    const sent = new Set<string>();

    return {
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
    };
}
