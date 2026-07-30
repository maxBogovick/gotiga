import { api } from '$lib/api';
import { SITE_URL } from '$lib/site';
import type { FigurineListItem } from '$lib/types/api';

// LLM-facing site index (https://llmstxt.org). Prerendered to a static /llms.txt in
// the web build so language-model crawlers get a clean, link-rich map of the archive
// without executing JS. The dev/SPA profile has no SEO surface, so it's excluded.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

// Collapse whitespace/newlines so each work stays a single tidy bullet line.
function oneLine(s: string | null | undefined): string {
    return (s ?? '').replace(/\s+/g, ' ').trim();
}

export async function GET({ fetch }: { fetch: typeof globalThis.fetch }) {
    let figurines: FigurineListItem[] = [];
    try {
        const all = await api.getAllFigurines(undefined, fetch);
        figurines = all.filter((f) => f.status !== 'in_progress');
    } catch {
        figurines = [];
    }

    const works = figurines
        .map((f) => {
            // FigurineListItem carries no description, so synthesise a short descriptor
            // from the structured facets that are available.
            const facets = [f.series, f.technique, f.material, f.year ? String(f.year) : null]
                .map((v) => oneLine(v))
                .filter(Boolean)
                .join(', ');
            const line = `- [${oneLine(f.name)}](${SITE_URL}/figurines/${f.slug ?? f.id})`;
            return facets ? `${line}: ${facets}` : line;
        })
        .join('\n');

    const body =
        `# Ritunia\n\n` +
        `> An author's cabinet of gothic, handmade miniatures and art dolls. ` +
        `Ritunia is a showcase archive — not a shop — where each piece is a unique, ` +
        `hand-sculpted and hand-painted figure with its own story.\n\n` +
        `## About\n\n` +
        `- [The author](${SITE_URL}/author): who makes the figures and why.\n` +
        `- [The workshop](${SITE_URL}/workshop): tools, materials and process.\n` +
        `- [Commission a piece](${SITE_URL}/commission): how to request a custom figure.\n\n` +
        `## Works\n\n` +
        (works || '- (the archive is currently quiet)') +
        `\n\n## Browse\n\n` +
        `- [Full archive](${SITE_URL}/figurines): every available, reserved and sold piece.\n` +
        `- [Upcoming](${SITE_URL}/upcoming): figures currently in progress.\n`;

    return new Response(body, {
        headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    });
}
