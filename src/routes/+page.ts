import { api, resolveMediaUrl } from '$lib/api';
import { SITE_URL, toAbsoluteUrl } from '$lib/site';
import { heroImageUrl, pickHeroFigurine, sortWorks, visibleWorks } from '$lib/home-hero';
import type { FigurineListItem, GazetteHome, HomeContent } from '$lib/types/api';

// Prerendered to static HTML in the web build so crawlers get real <head> meta
// (incl. the OG image). Matches the figurines archive page. Dev stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

const FALLBACK_HERO = '/images/cabinet-bg.jpeg';

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

// Everything the page's first screen is made of is resolved HERE: the background, the
// editable copy, the work the hero shows, and the works of the reel. So the static HTML
// carries the page — the headline photo the browser should fetch first, and a wall of real
// work with real names for anything that doesn't run JavaScript.
//
// This used to return no works at all, on the reasoning that a prerendered page bakes
// whatever load() returns and "Svelte does not re-set an image's src/srcset during
// hydration", so a seeded reel would wear the build day's photos. The first half is true;
// the conclusion was not. That exact problem is what AppImage repairs after hydration
// (see hydrate-image.ts) — and the rest of the site had been relying on it all along: the
// /figurines archive prerenders twenty photographs into its HTML, and all 39 figurine pages
// prerender their own. Only the home page abstained, and paid for it by shipping a hero
// with nothing behind it: no works, no names, nothing for a crawler to read on the site's
// front door, and a visible pop as the reel filled in after hydration.
//
// The freshness cost is the same one the archive already accepts: the HTML is a snapshot of
// the collection on the day of the deploy, and the browser corrects it on hydration (load()
// re-runs against the live API; text updates, and AppImage writes the right photos through).
// A crawler that never runs JS sees the build's collection — which is the whole catalogue,
// minus whatever was added since. That is strictly more than nothing.
export const load = async ({ fetch }: { fetch: typeof globalThis.fetch }) => {
    // `fetch` is passed to the api so the BROWSER uses SvelteKit's fetch (no window.fetch
    // warning); during prerender webFetch ignores it and uses the global fetch (see webFetch).
    const [bg, homeContent, page, author, gazette] = await Promise.all([
        api.getMainBackground(fetch).catch(() => null as string | null),
        api.getHomeContent(fetch).catch(() => DEFAULT_HOME_CONTENT),
        api.getFigurinesPage(undefined, fetch).catch(() => ({ items: [] as FigurineListItem[], total: 0 })),
        api.getAuthorProfile(fetch).catch(() => null),
        api.getGazetteHome(fetch).catch(() => ({ leaves: [], cuttings: [] } as GazetteHome)),
    ]);

    const works = sortWorks(visibleWorks(page.items));
    // The in-progress works are in the payload already (the list is the whole visible
    // collection; visibleWorks() is what holds them out of the reel), so the "N in the
    // making" figure needs no request of its own — and it now reaches the static HTML.
    const inProgress = sortWorks(page.items.filter((f) => f.status === 'in_progress'));
    const heroFig = pickHeroFigurine(works, homeContent);
    const heroImage = heroImageUrl(bg, heroFig, FALLBACK_HERO);

    // The OG card shows what the page shows.
    let ogImage = `${SITE_URL}${FALLBACK_HERO}`;
    try {
        const absolute = toAbsoluteUrl(resolveMediaUrl(heroImage));
        if (absolute) ogImage = absolute;
    } catch {
        // Keep the bundled fallback image.
    }

    return { ogImage, bg, homeContent, heroFig, author, works, inProgress, gazette };
};
