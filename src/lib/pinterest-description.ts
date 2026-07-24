/**
 * Deterministic Pinterest SEO copy generator for feed.xml — no network call, no
 * API key, works offline. Writes real sentences (Pinterest's own SEO guidance:
 * 2-4 sentences, primary keyword in the first ~100 characters, a stated
 * audience/benefit — not a keyword-stuffed fragment chain, which Pinterest's
 * algorithm treats as spam). Distinct from the work's public shortText (which
 * stays in the site's quiet museum-label voice). Always English: the public
 * brand ("Ritunia") is English-only, regardless of the admin's UI language.
 */

interface PinterestSource {
	name: string;
	shortText?: string | null;
	material?: string | null;
	technique?: string | null;
	dimensions?: string | null;
	year?: number | null;
}

interface Theme {
	match: RegExp;
	/** Short noun phrase carrying the primary keyword, e.g. "witch art doll". */
	noun: string;
	/** Audience/benefit clause — what a searcher is actually looking for. */
	hook: string;
}

// Ordered by specificity — first match wins, so a name like "Baba Yaga" hits
// the folklore rule before the generic "witch" one below it.
const THEME_RULES: Theme[] = [
	{ match: /baba\s*yaga/i, noun: 'Baba Yaga art doll', hook: 'a Slavic folklore piece for witch and fairy-tale collectors' },
	{ match: /frankenstein/i, noun: "Frankenstein's creature art doll", hook: 'a gothic literary classic for horror and dark-fantasy collectors' },
	{ match: /quasimodo|esmeralda|notre[- ]?dame/i, noun: 'Notre-Dame de Paris art doll', hook: 'a Victor Hugo classic for literary character collectors' },
	{ match: /hamlet/i, noun: 'Shakespearean art doll', hook: 'a literary classic for theatre and book lovers' },
	{ match: /musketeer|porthos/i, noun: 'literary character art doll', hook: 'a classic adventure-novel character for collectors' },
	{ match: /\bpoe\b|edgar allan/i, noun: 'Edgar Allan Poe inspired art doll', hook: 'a gothic literary piece for dark academia and horror fans' },
	{ match: /red riding hood|pied piper/i, noun: 'fairy tale art doll', hook: 'a storybook character for fairy-tale and folklore collectors' },
	{ match: /vampire/i, noun: 'vampire art doll', hook: 'a gothic collectible for Halloween and dark-fantasy fans' },
	{ match: /mermaid/i, noun: 'mermaid art doll', hook: 'a fantasy sea-creature collectible for mermaid lovers' },
	{ match: /steampunk/i, noun: 'steampunk art doll', hook: 'a fantasy collectible for steampunk and sci-fi fans' },
	{ match: /witch|sorceress|spell\s*book/i, noun: 'witch art doll', hook: 'a Halloween and dark-fantasy collectible for witch lovers' },
	{ match: /wizard|sorcerer|mage\b/i, noun: 'wizard art doll', hook: 'a fantasy collectible for wizard and magic lovers' },
	{ match: /gnome/i, noun: 'gnome art doll', hook: 'a cozy fantasy collectible for gnome and cottagecore lovers' },
	{ match: /\bbook\b/i, noun: 'miniature book sculpture', hook: 'a bookshelf collectible for book lovers and book-nook decor' },
	{ match: /ghost|spirit|specter|spectre/i, noun: 'gothic spirit art doll', hook: 'a dark-fantasy collectible for gothic decor lovers' },
	{ match: /grandma|granny|village|cottage/i, noun: 'cottagecore folk art doll', hook: 'a cozy rustic collectible for cottagecore lovers' },
	{ match: /fairy\s*tale|folklore/i, noun: 'fairy tale art doll', hook: 'a storybook collectible for fairy-tale lovers' },
];

const GENERIC_THEME: Theme = {
	match: /(?:)/,
	noun: 'OOAK art doll',
	hook: 'a unique handmade collectible for art doll lovers',
};

// Pinterest caps Pin descriptions around 500 chars; stay under it, matching
// the server-side feed generator's own cap.
const MAX_DESCRIPTION = 480;

function findTheme(source: PinterestSource): Theme {
	const haystack = `${source.name} ${source.shortText ?? ''}`;
	return THEME_RULES.find((rule) => rule.match.test(haystack)) ?? GENERIC_THEME;
}

/** The primary keyword phrase alone — used for previews or other short-form needs. */
export function detectPinterestTheme(source: PinterestSource): string {
	return findTheme(source).noun;
}

// The material is real content, not decoration — claiming "polymer clay" for a
// work actually sculpted from something else would mismatch what a Pinterest
// search on that material expects to find, which hurts relevance rather than
// helping it.
function craftSentence(source: PinterestSource): string {
	const material = source.material?.trim();
	const technique = source.technique?.trim();
	const base = material ? `hand-sculpted from ${material.toLowerCase()}` : 'hand-sculpted from polymer clay';
	const finish = technique ? technique.toLowerCase() : 'hand-painted and hand-dressed';
	return `${base}, ${finish}`;
}

export function generatePinterestDescription(source: PinterestSource): string {
	const name = source.name.trim();
	const { noun, hook } = findTheme(source);

	const sentences = [
		`${name} — a one-of-a-kind ${noun}, ${craftSentence(source)} by Ritunia.`,
		`This OOAK collectible figurine is ${hook}.`,
	];
	const details: string[] = [];
	if (source.dimensions?.trim()) details.push(source.dimensions.trim());
	if (source.year) details.push(String(source.year));
	if (details.length) sentences.push(`${details.join(', ')}.`);
	sentences.push('Original and never repeated — only one exists.');

	let desc = sentences.join(' ');
	if (desc.length > MAX_DESCRIPTION) {
		desc = desc.slice(0, MAX_DESCRIPTION - 1).trimEnd() + '…';
	}
	return desc;
}
