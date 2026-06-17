import { writable, derived } from 'svelte/store';

// ── Reader's typeface preference ─────────────────────────────────────────────
// A personal choice — which serif the catalogue's prose is "set in". Layered on
// top of the admin theme via a dedicated --font-reading var, so it never clobbers
// the admin-controlled --font-serif. Every face below ships Cyrillic (RU + EN).

export interface ReadingFont {
    id: string;
    /** Shown in the colophon and menu, e.g. "набрано шрифтом · Garamond". */
    name: string;
    /** One-word character note for the menu. */
    note: string;
    /** Full CSS font stack assigned to --font-reading. */
    stack: string;
    /** Google Fonts href, lazily injected on first use. null = already in app.html. */
    href: string | null;
}

export const READING_FONTS: ReadingFont[] = [
    {
        id: 'garamond',
        name: 'Garamond',
        note: 'renaissance',
        stack: "'EB Garamond', Georgia, serif",
        href: null, // loaded statically in app.html as the default
    },
    {
        id: 'cormorant',
        name: 'Cormorant',
        note: 'high contrast',
        stack: "'Cormorant Garamond', Georgia, serif",
        href: 'https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,400;0,500;1,400&display=swap',
    },
    {
        id: 'newsreader',
        name: 'Newsreader',
        note: 'bookish',
        stack: "'Newsreader', Georgia, serif",
        href: 'https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,500;1,6..72,400&display=swap',
    },
    {
        id: 'spectral',
        name: 'Spectral',
        note: 'calm',
        stack: "'Spectral', Georgia, serif",
        href: 'https://fonts.googleapis.com/css2?family=Spectral:ital,wght@0,400;0,500;1,400&display=swap',
    },
    {
        id: 'lora',
        name: 'Lora',
        note: 'warm',
        stack: "'Lora', Georgia, serif",
        href: 'https://fonts.googleapis.com/css2?family=Lora:ital,wght@0,400;0,500;1,400&display=swap',
    },
    {
        id: 'yeseva',
        name: 'Yeseva One',
        note: 'expressive',
        stack: "'Yeseva One', Georgia, serif",
        href: 'https://fonts.googleapis.com/css2?family=Yeseva+One&display=swap',
    },
    {
        id: 'caveat',
        name: 'Caveat',
        note: 'by hand',
        stack: "'Caveat', 'Segoe Script', cursive",
        href: 'https://fonts.googleapis.com/css2?family=Caveat:wght@400;500;600&display=swap',
    },
];

const DEFAULT_ID = 'garamond';
const STORAGE_KEY = 'gotiga_reading_font';

function findFont(id: string | null): ReadingFont {
    return READING_FONTS.find((f) => f.id === id) ?? READING_FONTS[0];
}

function getInitialId(): string {
    if (typeof window === 'undefined') return DEFAULT_ID;
    const saved = localStorage.getItem(STORAGE_KEY);
    return READING_FONTS.some((f) => f.id === saved) ? (saved as string) : DEFAULT_ID;
}

function ensureFontLoaded(font: ReadingFont): void {
    if (typeof document === 'undefined' || !font.href) return;
    if (document.querySelector(`link[data-reading-font="${font.id}"]`)) return;
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = font.href;
    link.setAttribute('data-reading-font', font.id);
    document.head.appendChild(link);
}

function apply(font: ReadingFont): void {
    if (typeof document === 'undefined') return;
    ensureFontLoaded(font);
    document.documentElement.style.setProperty('--font-reading', font.stack);
}

export const readingFontId = writable<string>(getInitialId());

export const readingFont = derived(readingFontId, ($id) => findFont($id));

// The committed choice — what the page reverts to when a hover preview ends.
let committedId = getInitialId();

// Persist + apply on every change (also fires once on store creation in the browser).
readingFontId.subscribe((id) => {
    if (typeof window === 'undefined') return;
    const font = findFont(id);
    committedId = font.id;
    localStorage.setItem(STORAGE_KEY, font.id);
    apply(font);
});

export function setReadingFont(id: string): void {
    readingFontId.set(id);
}

/** Temporarily render the prose in `id`'s face — for hovering a menu option. */
export function previewReadingFont(id: string): void {
    apply(findFont(id));
}

/** Drop any hover preview and restore the committed choice. */
export function endReadingFontPreview(): void {
    apply(findFont(committedId));
}

/** Lazily load every face so the menu can preview each in its own typeface. */
export function preloadReadingFonts(): void {
    for (const font of READING_FONTS) ensureFontLoaded(font);
}
