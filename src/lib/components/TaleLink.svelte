<script lang="ts">
  /**
   * A link to this house, written into a tale.
   *
   * Two forms, chosen by how the author wrote it. Inside a sentence it is
   * only the name of what it leads to, in italics with a hairline under it:
   * no thumbnail, no hover card — anything else in the line moves the text
   * around it. On a line of its own the author set it apart, so it stands
   * apart: a tipped-in card with the arch, the kind of room, the name and a
   * line of its description. The card is a block and may use `AppImage`.
   *
   * The card depends on `ArchClip` being on the page (`#tale-arch`).
   */
  import { t, lang } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n';
  import { leafCopy } from '$lib/gazette';
  import { PAGE_LABELS, type SiteRef, type SiteRefInfo } from '$lib/siteLinks';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    ref,
    info,
    card = false,
    morph = '',
  }: {
    ref: SiteRef;
    info?: SiteRefInfo;
    card?: boolean;
    /** `view-transition-name` for the card's photograph; empty — none. */
    morph?: string;
  } = $props();

  const KICKERS: Record<SiteRef['room'], TranslationKey> = {
    work: 'talesLinkWork',
    tale: 'talesLinkTale',
    leaf: 'talesLinkLeaf',
    page: 'talesLinkPage',
  };

  let named = $derived.by(() => {
    if (info?.room === 'work') return { title: info.name, dek: info.dek };
    if (info) {
      const copy = leafCopy(info.leaf, $lang);
      return { title: copy.title, dek: copy.dek };
    }
    if (ref.room === 'page') {
      const key = PAGE_LABELS[ref.handle] as TranslationKey | undefined;
      if (key) return { title: $t(key), dek: '' };
    }
    // Nothing could name it: the address itself, as a reader would read it.
    return { title: ref.bare, dek: '' };
  });

  let image = $derived(info?.image ?? '');
  let kicker = $derived($t(KICKERS[ref.room]));
</script>

{#if card}
  <a class="tc" href={ref.href}>
    <span class="tc-arch" class:tc-arch--bare={!image} style={morph ? `view-transition-name: ${morph}` : undefined}>
      {#if image}
        <AppImage src={image} alt="" class="tc-img" sizes="84px" />
      {:else}
        <span class="tc-glyph" aria-hidden="true">✦</span>
      {/if}
    </span>
    <span class="tc-copy">
      <span class="tc-kicker">{kicker}</span>
      <span class="tc-title">{named.title}</span>
      {#if named.dek}<span class="tc-dek">{named.dek}</span>{/if}
    </span>
    <span class="tc-go" aria-hidden="true">→</span>
  </a>
{:else}
  <!-- One line on purpose: this sits inside a sentence, and a newline between
       the tags would print as a space inside the link. -->
  <a class="tl" href={ref.href}>{named.title}</a>
{/if}

<style>
  /* ── Inside a sentence ─────────────────────────────────────────────── */

  .tl {
    position: relative;
    color: var(--deep, #6f3b24);
    text-decoration: none;
    /* The underline is drawn, not declared: a hairline of copper that fills
       to ink on hover, so the link reads as part of the page's lettering
       rather than a browser's blue. */
    background-image: linear-gradient(currentColor, currentColor),
      linear-gradient(rgba(198, 95, 60, 0.45), rgba(198, 95, 60, 0.45));
    background-size: 0% 1px, 100% 1px;
    background-position: 0 100%, 0 100%;
    background-repeat: no-repeat;
    padding-bottom: 1px;
    transition: background-size 0.45s cubic-bezier(0.2, 0.8, 0.3, 1), color 0.25s;
    -webkit-box-decoration-break: clone;
    box-decoration-break: clone;
  }
  .tl:hover,
  .tl:focus-visible {
    color: var(--copper, #c65f3c);
    background-size: 100% 1px, 100% 1px;
  }

  .tl { font-style: italic; }

  /* ── On a line of its own ──────────────────────────────────────────── */

  .tc {
    display: flex;
    align-items: center;
    gap: clamp(16px, 2.4vw, 24px);
    max-width: 62ch;
    margin: 0.6em 0 1.8em;
    padding: 16px clamp(16px, 2.6vw, 26px) 16px 16px;
    text-decoration: none;
    color: inherit;
    background:
      radial-gradient(ellipse 80% 120% at 0% 50%, rgba(198, 95, 60, 0.07), transparent 70%),
      rgba(255, 252, 246, 0.55);
    border: 1px solid var(--line, #d8c6b1);
    outline: 1px solid rgba(216, 198, 177, 0.5);
    outline-offset: 4px;
    /* Tipped in, the way a card slipped between two pages sits — and it
       straightens when you reach for it. */
    transform: rotate(-0.6deg);
    transition:
      transform 0.4s cubic-bezier(0.2, 0.8, 0.3, 1),
      box-shadow 0.4s,
      border-color 0.25s;
  }
  .tc:hover,
  .tc:focus-visible {
    transform: rotate(0deg) translateY(-2px);
    border-color: rgba(198, 95, 60, 0.45);
    box-shadow: 0 12px 30px rgba(52, 37, 28, 0.12);
  }

  .tc-arch {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: clamp(64px, 9vw, 84px);
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: #1a120e;
    clip-path: url(#tale-arch);
  }
  .tc-arch--bare {
    background: linear-gradient(180deg, rgba(198, 95, 60, 0.18), rgba(111, 59, 36, 0.28));
  }
  .tc-arch :global(.app-image-wrap),
  .tc-arch :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .tc-glyph {
    padding-top: 30%;
    font-size: 20px;
    color: var(--copper, #c65f3c);
  }

  .tc-copy { display: grid; gap: 6px; min-width: 0; flex: 1; }
  .tc-kicker {
    font-family: Inter, system-ui, sans-serif;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
  }
  .tc-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(21px, 2.4vw, 27px);
    font-weight: 400;
    line-height: 1.12;
    text-wrap: balance;
    color: var(--ink, #34251c);
    transition: color 0.25s;
  }
  .tc:hover .tc-title { color: var(--deep, #6f3b24); }
  .tc-dek {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 15.5px;
    font-style: italic;
    line-height: 1.4;
    color: var(--muted, #5f4636);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .tc-go {
    flex-shrink: 0;
    font-size: 18px;
    color: var(--copper, #c65f3c);
    opacity: 0.55;
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.3, 1), opacity 0.25s;
  }
  .tc:hover .tc-go { transform: translateX(4px); opacity: 1; }

  @media (prefers-reduced-motion: reduce) {
    .tl, .tc, .tc-go { transition: none; }
    .tc, .tc:hover, .tc:focus-visible { transform: none; }
    .tc:hover .tc-go { transform: none; }
  }
</style>
