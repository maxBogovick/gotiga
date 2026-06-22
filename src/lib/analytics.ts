import { api, isTauri } from '$lib/api';
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

function basePayload(figurineId: string, pageViewId: string): Pick<
    AnalyticsEventPayload,
    'figurineId' | 'path' | 'referrer' | 'utmSource' | 'utmMedium' | 'utmCampaign' | 'pageViewId' | 'clientTs'
> {
    return {
        figurineId,
        path: `${location.pathname}${location.search}`,
        referrer: document.referrer || null,
        utmSource: utm('utm_source'),
        utmMedium: utm('utm_medium'),
        utmCampaign: utm('utm_campaign'),
        pageViewId,
        clientTs: new Date().toISOString(),
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
