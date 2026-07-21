// Generates a markdown twin next to every prerendered HTML page (build/foo.html →
// build/foo.md), so nginx can hand agents a markdown response instead of full page
// HTML when they send `Accept: text/markdown` (see nginx.conf's $md_ext map).
//
// Runs as a post-step after `vite build` (see package.json's build:web script) —
// intentionally NOT a Cloudflare Worker or any other edge/CDN-specific mechanism,
// since this site is a plain static SPA behind whatever proxy happens to sit in
// front of it; the twin files themselves are provider-agnostic.
//
// Only <main>'s subtree is converted — the root layout has exactly one <main>
// wrapping every route's actual content, as a SIBLING of <header>/<nav>/<footer>
// (never their ancestor), so scoping to it drops the site chrome for free.
// This matters because some pages have their OWN local <header> inside <main>
// for the article/section itself (e.g. /author's "<header><h1>Voice of the
// Author</h1></header>") — an earlier version of this script blindly stripped
// every <header>/<nav>/<footer> tag anywhere in the document and silently ate
// that heading along with the real site nav. Within <main>, only strip things
// that are unconditionally non-text regardless of nesting: script/style/
// noscript/svg/video/canvas. Deliberately NOT stripping [aria-hidden]
// elements in general — the home hero <h1> splits its visible text into
// aria-hidden spans for a per-letter CSS animation, with the real text living
// only in the parent's aria-label; a blanket aria-hidden removal silently eats
// that heading's only text.
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join, extname } from 'node:path';
import * as cheerio from 'cheerio';
import { NodeHtmlMarkdown } from 'node-html-markdown';

const BUILD_DIR = process.argv[2] || 'build';
const STRIP_SELECTORS = ['script', 'style', 'noscript', 'svg', 'video', 'canvas'];
// The SPA fallback shell has no prerendered content — a markdown twin of it would
// just be empty chrome.
const SKIP_FILES = new Set(['app.html']);

const nhm = new NodeHtmlMarkdown();
let count = 0;
let skippedNoMain = 0;
let failed = 0;

// A markdown twin is a nice-to-have on top of an already-successful HTML build,
// not a build-blocking requirement — this runs as a `RUN` step in
// Dockerfile.frontend, so an uncaught throw here would fail the whole image
// over what is, at worst, one page missing its agent-facing markdown variant
// (nginx already falls back to the .html file for that one page — see
// nginx.conf's location / comment). One bad file is logged and skipped; the
// rest of the batch still gets its twins.
async function convert(htmlPath) {
	try {
		const html = await readFile(htmlPath, 'utf8');
		const $ = cheerio.load(html);
		const main = $('main').first();
		if (main.length === 0) {
			skippedNoMain++;
			return;
		}
		for (const selector of STRIP_SELECTORS) main.find(selector).remove();
		const markdown = nhm.translate(main.html() ?? '').replace(/\n{3,}/g, '\n\n').trim();

		const mdPath = htmlPath.replace(/\.html$/, '.md');
		await writeFile(mdPath, markdown + '\n', 'utf8');
		count++;
	} catch (err) {
		failed++;
		console.warn(`⚠ skipped ${htmlPath}: ${err.message}`);
	}
}

async function walk(dir) {
	const entries = await readdir(dir, { withFileTypes: true });
	await Promise.all(
		entries.map((entry) => {
			const full = join(dir, entry.name);
			if (entry.isDirectory()) return walk(full);
			if (entry.isFile() && extname(entry.name) === '.html' && !SKIP_FILES.has(entry.name)) {
				return convert(full);
			}
		})
	);
}

console.log(`→ generating markdown twins under ${BUILD_DIR}/`);
await walk(BUILD_DIR);
console.log(`✓ ${count} markdown file(s) generated`);
if (skippedNoMain) console.log(`⚠ ${skippedNoMain} HTML file(s) had no <main> — skipped`);
if (failed) console.log(`⚠ ${failed} HTML file(s) failed to convert — skipped, see warnings above`);
