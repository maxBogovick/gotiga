// Post-step after `vite build` (package.json's build:web): re-enables
// stylesheets that SvelteKit marked as "already inlined" when they are not.
//
// With `inlineStyleThreshold` SvelteKit puts a page's small stylesheets into a
// <style> in the HTML and leaves the original <link> in a disabled state
// (`disabled media="(max-width: 0)"`), so the browser never loads the file.
// It decides what to inline by index within the node's stylesheet list, from
// the *server* build. A component the server never renders — one that appears
// only after mount, like the scene beside a tale or a modal — is tree-shaken
// out of the server bundle, its CSS goes with it, and the client file is
// still marked inlined: the page is prerendered without the styles and the
// browser is told not to fetch them. Every class hash in such a file is then
// missing from the inline <style>.
//
// The check is the one that matters: a disabled link whose file carries a
// `svelte-xxxx` hash absent from the page's inline <style> is not inlined,
// whatever the attributes say. Such a link becomes an ordinary stylesheet.
// The file is a little bigger than it had to be and the page is right.
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join, dirname, resolve } from 'node:path';

const BUILD_DIR = resolve(process.argv[2] || 'build');
const LINK = /<link href="([^"]+)" rel="stylesheet" disabled media="\(max-width: 0\)">/g;
const STYLE = /<style[^>]*>([\s\S]*?)<\/style>/g;
const HASH = /svelte-[a-z0-9]+/g;

async function* walk(dir) {
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name !== '_app') yield* walk(p);
    } else if (e.name.endsWith('.html')) yield p;
  }
}

const hashesOf = new Map();
async function cssHashes(file) {
  if (!hashesOf.has(file)) {
    let set = new Set();
    try {
      set = new Set((await readFile(file, 'utf8')).match(HASH) ?? []);
    } catch {
      // A link to a file that is not there: nothing to compare, leave it.
    }
    hashesOf.set(file, set);
  }
  return hashesOf.get(file);
}

let pages = 0;
let relinked = 0;
for await (const file of walk(BUILD_DIR)) {
  const html = await readFile(file, 'utf8');
  const inline = [...html.matchAll(STYLE)].map((m) => m[1]).join('\n');
  const jobs = [...html.matchAll(LINK)];
  if (!jobs.length) continue;

  let out = html;
  let changed = false;
  for (const [tag, href] of jobs) {
    const hashes = await cssHashes(resolve(dirname(file), href));
    const missing = [...hashes].filter((h) => !inline.includes(h));
    if (!hashes.size || !missing.length) continue;
    out = out.replace(tag, `<link href="${href}" rel="stylesheet">`);
    relinked++;
    changed = true;
  }
  if (changed) {
    await writeFile(file, out);
    pages++;
  }
}
console.log(`→ fix-inlined-css: ${relinked} stylesheet link(s) re-enabled on ${pages} page(s)`);
