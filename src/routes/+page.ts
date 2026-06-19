import { api, resolveMediaUrl } from '$lib/api';
import { SITE_URL, toAbsoluteUrl } from '$lib/site';

// Prerendered to static HTML in the web build so crawlers get real <head> meta
// (incl. the OG image). Matches the figurines archive page. Tauri stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

const FALLBACK_OG = `${SITE_URL}/images/cabinet-room.jpg`;

export const load = async () => {
    let ogImage = FALLBACK_OG;
    try {
        const bg = await api.getMainBackground();
        const resolved = resolveMediaUrl(bg);
        const absolute = toAbsoluteUrl(resolved);
        if (absolute) ogImage = absolute;
    } catch {
        // Keep the bundled fallback image.
    }
    return { ogImage };
};
