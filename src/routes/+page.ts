import { api, resolveMediaUrl } from '$lib/api';
import { SITE_URL, toAbsoluteUrl } from '$lib/site';
import type { HomeContent } from '$lib/types/api';

// Prerendered to static HTML in the web build so crawlers get real <head> meta
// (incl. the OG image). Matches the figurines archive page. Tauri stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

const FALLBACK_OG = `${SITE_URL}/images/cabinet-bg.jpeg`;

const DEFAULT_HOME_CONTENT: HomeContent = {
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

// The background and the editable home copy are fetched here so the hero — the
// LCP element — is ready the moment the page component exists, instead of waiting
// on a client fetch that only starts once the whole tree has mounted.
//
// The list of works is deliberately NOT fetched here. This page is PRERENDERED in
// the web build, so anything load() returns is frozen into static HTML at build
// time: the reel's cards, and with them their <img srcset>, would be a snapshot of
// the collection as it stood on the day of the deploy. Svelte does not re-set an
// image's src/srcset during hydration, and the reel's order is date-dependent
// (dailyRotate), so the live data would land on DOM nodes carrying the build's
// photos — a card titled "Gnome" wearing the photo of whatever work occupied that
// slot at build time. The works are therefore loaded on the client (see init() in
// +page.svelte), where every change reaches the DOM normally.
//
// The same reasoning does NOT bite the background or the copy: the background's URL
// is a fixed name (cabinet-bg.{ext}, served no-cache), and text nodes ARE updated on
// hydration — homeContent is re-fetched in init() anyway.
export const load = async () => {
    const [bg, homeContent] = await Promise.all([
        api.getMainBackground().catch(() => null as string | null),
        api.getHomeContent().catch(() => DEFAULT_HOME_CONTENT),
    ]);

    let ogImage = FALLBACK_OG;
    try {
        const resolved = resolveMediaUrl(bg);
        const absolute = toAbsoluteUrl(resolved);
        if (absolute) ogImage = absolute;
    } catch {
        // Keep the bundled fallback image.
    }
    return { ogImage, bg, homeContent };
};
