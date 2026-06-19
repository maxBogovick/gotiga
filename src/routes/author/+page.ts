import { api } from '$lib/api';
import type { AuthorText, AuthorProfile } from '$lib/types/api';

// Web build: prerender to static HTML so crawlers and LLMs see the author's bio and
// notes (previously fetched in onMount → invisible to non-JS bots). Tauri stays SPA.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

export const load = async () => {
    // Best-effort: an unreachable API must not fail the whole prerender build — the
    // page degrades to its empty state, exactly as the old onMount catch did.
    const [texts, profile] = await Promise.all([
        api.getAuthorTexts().catch(() => [] as AuthorText[]),
        api.getAuthorProfile().catch(() => null as AuthorProfile | null),
    ]);
    return { texts, profile };
};
