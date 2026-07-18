<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import AppImage from '$lib/components/AppImage.svelte';
  import { t } from '$lib/i18n';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import type { FigurineListItem } from '$lib/types/api';
  import { figurineHref } from '$lib/figurineHref';

  type Props = {
    fig: FigurineListItem;
    /** 1-based position — printed as the 01 / 02 kicker. */
    index: number;
    /** The work's own short text; falls back to its museum attributes. */
    story?: string | null;
    /** Odd cards mirror, so the eye zig-zags down the list. */
    flip?: boolean;
    /** Which named home-page block rendered this card (e.g. "home_grid") —
     * tags the outgoing link so admin analytics can attribute the click.
     * Optional — this component is only used by the home page today. */
    source?: string;
  };

  let { fig, index, story = null, flip = false, source }: Props = $props();

  let text = $derived(
    story?.trim() ||
      [fig.series, fig.technique, fig.material, fig.dimensions]
        .map((v) => v?.trim())
        .filter(Boolean)
        .join(' · ')
  );

  const pad = (n: number) => String(n).padStart(2, '0');

  const ROMAN_MAP: [number, string][] = [
    [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
    [100, 'C'], [90, 'XC'], [50, 'L'], [40, 'XL'],
    [10, 'X'], [9, 'IX'], [5, 'V'], [4, 'IV'], [1, 'I'],
  ];

  function toRoman(num: number): string {
    let n = num;
    let out = '';
    for (const [value, sym] of ROMAN_MAP) {
      while (n >= value) { out += sym; n -= value; }
    }
    return out || 'I';
  }

  let romanNumeral = $derived(toRoman(index));

  // Cover-crop focal point (normalised 0..1, default centre) — keeps the subject
  // framed when the photo is cropped to fill the plate.
  let focalPos = $derived(
    `${Math.min(1, Math.max(0, fig.focalX ?? 0.5)) * 100}% ${Math.min(1, Math.max(0, fig.focalY ?? 0.5)) * 100}%`
  );

  // Recently catalogued — a quiet wax mark, not a sales badge.
  let isNew = $derived.by(() => {
    if (!fig.createdAt) return false;
    const ts = new Date(fig.createdAt).getTime();
    if (Number.isNaN(ts)) return false;
    return Date.now() - ts < 21 * 24 * 60 * 60 * 1000;
  });

  let statusLabel = $derived(
    fig.status === 'available'
      ? $t('archiveStatusAvailableLabel')
      : fig.status === 'reserved'
        ? $t('archiveStatusReservedLabel')
        : fig.status === 'in_progress'
          ? $t('profileWishInProgress')
          : $t('archiveStatusSoldLabel')
  );

  // ── Card actions: mark, share, commission. Same store and the same wording
  // the archive plates use (HomeFigurineTile), so a work marked here is marked
  // everywhere and the copy stays in one place.
  let href = $derived(`/figurines/${fig.id}`);
  let saved = $derived(savedFigurines.has(fig.id));
  let justSaved = $state(false);
  let copied = $state(false);
  let pulseTimer: ReturnType<typeof setTimeout> | undefined;
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => {
    savedFigurines.load();
    return () => {
      clearTimeout(pulseTimer);
      clearTimeout(copyTimer);
    };
  });

  function toggleSaved() {
    savedFigurines.toggle(fig.id);
    if (savedFigurines.has(fig.id)) {
      justSaved = true;
      clearTimeout(pulseTimer);
      pulseTimer = setTimeout(() => { justSaved = false; }, 650);
    }
  }

  async function shareWork() {
    const url = typeof location !== 'undefined' ? `${location.origin}${href}` : href;
    try {
      if (typeof navigator !== 'undefined' && navigator.share) {
        await navigator.share({ title: fig.name, url });
        return;
      }
      await navigator.clipboard.writeText(url);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = false; }, 1800);
    } catch {
      /* the visitor dismissed the share sheet — nothing to do */
    }
  }

  function openSimilarCommission() {
    goto(`/commission?source=${encodeURIComponent(fig.id)}`);
  }
</script>

<!-- The `glass-work` class is what the admin's reel theme targets when it emits
     the work-pane variables; the visual classes are the local ones. -->
<article class="card glass-work" class:card-flip={flip} aria-labelledby="card-{fig.id}">
  <div class="body">
    <span class="num">
      {pad(index)}
      <span class="num-rule" aria-hidden="true"></span>
    </span>

    <h3 id="card-{fig.id}" class="title">{fig.name}</h3>

    {#if text}
      <p class="text">{text}</p>
    {/if}

    <div class="actions">
      <a class="cta" href={href}>
        <span>{$t('reelDiscover')}</span>
        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
          <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1" />
        </svg>
      </a>

      <div class="quick">
        <button
          class="qbtn"
          class:is-active={saved}
          class:just-saved={justSaved}
          onclick={toggleSaved}
          aria-pressed={saved}
          aria-label={saved ? $t('cardSaved') : $t('cardSave')}
          title={saved ? $t('cardSaved') : $t('cardSave')}
        >
          <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path
              d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
              fill={saved ? 'currentColor' : 'none'}
              stroke="currentColor"
              stroke-width="1.1"
              stroke-linejoin="round"
            />
          </svg>
        </button>

        <button
          class="qbtn"
          class:is-active={copied}
          onclick={shareWork}
          aria-label={copied ? $t('cardLinkCopied') : $t('cardShare')}
          title={copied ? $t('cardLinkCopied') : $t('cardShare')}
        >
          {#if copied}
            <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <path d="M2.5 7.5L5.5 10.5L11.5 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          {:else}
            <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
              <circle cx="3" cy="7" r="1.6" stroke="currentColor" stroke-width="1.1"/>
              <circle cx="11" cy="3" r="1.6" stroke="currentColor" stroke-width="1.1"/>
              <circle cx="11" cy="11" r="1.6" stroke="currentColor" stroke-width="1.1"/>
              <path d="M4.4 6.2L9.6 3.6M4.4 7.8L9.6 10.4" stroke="currentColor" stroke-width="1.1"/>
            </svg>
          {/if}
        </button>

        {#if fig.status === 'available'}
          <!-- the commission funnel, kept alive on every plate: "make me one like this" -->
          <button
            class="qbtn"
            onclick={openSimilarCommission}
            aria-label={$t('commissionCreateSimilarCta')}
            title={$t('commissionCreateSimilarCta')}
          >
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M5 2h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5"/>
              <path d="M5 2a2 2 0 0 0 0 4"/>
              <path d="M5 6a2 2 0 0 1 0 4"/>
              <path d="M5 10a2 2 0 0 0 0 4"/>
              <line x1="8" y1="5.5" x2="12" y2="5.5"/>
              <line x1="8" y1="8" x2="12" y2="8"/>
              <line x1="8" y1="10.5" x2="10.5" y2="10.5"/>
            </svg>
          </button>
        {/if}
      </div>
    </div>
  </div>

  <a
    class="photo"
    href={figurineHref(fig, source)}
    style="--focal:{focalPos}"
    aria-label="{$t('homeViewFigurine')}: {fig.name}"
  >
    <!-- The plate is ~536 CSS px wide on a wide screen and full-bleed on a phone. `sizes`
         says exactly that, and AppImage offers the 420/900/1800 renditions, so the browser
         resolves the right one itself: the 900px medium on a phone (where the 1800px
         preview used to be pulled — ~390 KB to paint a 390 px-wide screen), the 1800px
         preview on a retina desktop, where the plate really is over 1000 real pixels. -->
    <AppImage
      src={fig.faceImageLargeUrl ?? fig.faceImageUrl}
      thumbUrl={fig.thumbUrl ?? fig.faceImageUrl}
      alt={fig.name}
      class="photo-img"
      sizes="(max-width: 680px) 100vw, 536px"
    />

    {#if fig.detailImageUrl}
      <!-- A second angle, held in reserve for a lingering look — not a product
           swatch swap, a closer look at the same piece. -->
      <AppImage
        src={fig.detailImageLargeUrl ?? fig.detailImageUrl}
        alt=""
        class="photo-img-alt"
        sizes="(max-width: 680px) 100vw, 536px"
      />
    {/if}

    <!-- Raking light: a slow sheen travelling across the plate on hover, the way
         light moves over a thing you pick up and turn. -->
    <span class="photo-rake" aria-hidden="true"></span>

    <!-- The museum plate: what it is, when, and its number in the wall. -->
    <span class="plate">
      <span class="plate-status status-{fig.status}">{statusLabel}</span>
      {#if fig.year}
        <span class="plate-dot" aria-hidden="true">·</span>
        <span class="plate-year">{fig.year}</span>
      {/if}
      <span class="plate-roman">{romanNumeral}</span>
    </span>

    <span class="marks">
      {#if fig.houseFavorite}
        <span class="mark mark-favorite">{$t('houseFavoriteBadge')}</span>
      {/if}
      {#if isNew}
        <span class="mark mark-new">{$t('archiveCardNew')}</span>
      {/if}
    </span>
  </a>
</article>

<style>
  /* ══════════════════════════════════════════════════════════════════
     One work, as a card of warm paper.

     This sits on the site's own parchment, so it is NOT glass: frosting is a
     blur of what is behind a pane, and behind this there is only flat colour —
     there would be nothing to blur. What reads as quality on paper is different:
     a real edge, a shadow with two layers (a tight contact shadow and a wide
     soft one), and type with room around it.

     Every value still comes from the admin's reel theme (--card-*) so the panel
     keeps working; the fallbacks below are the designed look.
     ══════════════════════════════════════════════════════════════════ */
  .card {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1.1fr;
    align-items: center;
    gap: clamp(1.25rem, 2.5vw, 2.5rem);
    width: 100%;
    padding: clamp(1rem, 1.6vw, 1.4rem);

    border-radius: var(--card-radius, 18px);
    background: var(--card-bg, #fdfaf5);
    border: 1px solid var(--card-edge-color, rgba(160, 116, 74, 0.24));
    /* The plate and the marks carry a z-index of their own. Without a stacking
       context here they compete in the ROOT one, where they tie with the sticky
       section header (.context-hd, z-index 3) and — being later in the DOM — win,
       printing themselves over its copy as the card scrolls under it. Isolated,
       their z-index only orders them against each other, and the card as a whole
       passes beneath the header where it belongs. */
    isolation: isolate;
    /* Three layers, and each does one job: a hairline of light along the top edge
       (paper catches the light), a tight contact shadow that keeps the card ON the
       page, and a wide soft one that lifts it off. A single shadow reads either as
       a sticker or as a smudge. */
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.7),
      0 1px 2px rgb(var(--card-film, 60 25 10) / 0.06),
      0 10px 30px rgb(var(--card-film, 60 25 10) / calc(var(--card-shadow, 0.5) * 0.16));
    transition:
      transform 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      box-shadow 0.5s cubic-bezier(0.22, 1, 0.36, 1),
      border-color 0.5s ease;
  }

  /* The card itself does not answer the pointer. Only the photograph does — the
     paper it sits on stays still, the way paper does. */

  /* Odd cards mirror, so the eye zig-zags down the column instead of running a
     straight line. `direction` would flip the text too, so order the cells. */
  .card-flip .body { order: 2; }
  .card-flip .photo { order: 1; }

  .body {
    min-width: 0;
    padding: clamp(0.75rem, 1.5vw, 1.5rem);
  }

  /* ── The number: a plate mark, with a rule running off it ── */
  .num {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    font-family: var(--font-display);
    font-size: var(--card-meta-size, 0.82rem);
    letter-spacing: 0.22em;
    color: var(--card-meta, #a0745a);
  }

  .num-rule {
    display: block;
    width: 2.5rem;
    height: 1px;
    background: linear-gradient(
      90deg,
      var(--card-meta, #a0745a),
      transparent
    );
  }

  .title {
    margin-top: 1.1rem;
    color: var(--card-title, #2c1710);
    font-family: var(--font-display);
    font-weight: 400;
    /* The admin's size is the ceiling; the viewport still gets a say, or a big
       setting would burst the card on a phone. */
    font-size: clamp(1.5rem, 2.6vw, var(--card-title-size, 2.2rem));
    line-height: 1.12;
    letter-spacing: -0.015em;
    text-wrap: balance;
    /* Some works carry a whole sentence in the `name` field — display type at this
       size would run off the card. The detail page carries the rest. */
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
  }

  .text {
    margin-top: 0.9rem;
    max-width: 34ch;
    font-size: var(--card-body-size, 0.92rem);
    line-height: 1.75;
    color: var(--card-body, #5a3420);
    text-wrap: pretty;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
  }

  /* ── The ask, and the quiet actions that sit beside it ── */
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.9rem;
    margin-top: 1.6rem;
  }

  .quick {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  /* Engraved on the paper rather than floating over it: a hairline outline, no
     shadow, and the copper only arrives on intent. */
  .qbtn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 1px solid var(--card-edge-color, rgba(160, 116, 74, 0.32));
    border-radius: 3px;
    background: transparent;
    color: var(--card-meta, #a0745a);
    cursor: pointer;
    transition: color 0.24s ease, border-color 0.24s ease, background 0.24s ease, transform 0.14s ease;
  }

  .qbtn:hover {
    border-color: var(--card-edge-hover, rgba(192, 88, 44, 0.55));
    background: rgba(192, 88, 44, 0.06);
    color: var(--copper, #c65f3c);
  }

  .qbtn:active { transform: scale(0.94); }

  .qbtn.is-active {
    color: var(--copper, #c65f3c);
    border-color: rgba(192, 88, 44, 0.55);
  }

  .qbtn.just-saved { animation: heart-pop 0.62s cubic-bezier(0.34, 1.56, 0.64, 1); }

  .qbtn:focus-visible {
    outline: 2px solid rgba(192, 88, 44, 0.55);
    outline-offset: 2px;
  }

  @keyframes heart-pop {
    0% { transform: scale(1); }
    35% { transform: scale(1.32); }
    60% { transform: scale(0.92); }
    100% { transform: scale(1); }
  }

  .cta {
    position: relative;
    overflow: hidden;
    display: inline-flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.85rem 1.6rem;
    border-radius: var(--card-btn-radius, 4px);
    border: 1px solid var(--card-btn-fill, #2c1710);
    background: var(--card-btn-fill, #2c1710);
    color: var(--card-btn-text, #faf6ee);
    font-size: var(--card-btn-size, 0.68rem);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    transition: background 0.4s ease, color 0.4s ease;
  }

  .cta span,
  .cta svg {
    position: relative;
    z-index: 1;
  }

  .cta svg {
    transition: transform 0.45s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .cta:hover svg,
  .cta:focus-visible svg {
    transform: translateX(4px);
  }

  /* Light sweeps across on the way in. */
  .cta::before {
    content: '';
    position: absolute;
    inset: 0;
    transform: translateX(-110%) skewX(-18deg);
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.22), transparent);
    transition: transform 0.75s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .cta:hover::before {
    transform: translateX(110%) skewX(-18deg);
  }

  /* ── The photograph: the reason the card exists ── */
  .photo {
    /* An <a>, so it must be told to be a box at all — inline elements ignore
       aspect-ratio. */
    display: block;
    position: relative;
    margin: 0;
    border-radius: calc(var(--card-radius, 18px) * 0.72);
    overflow: hidden;
    /* The subject is a standing figure, so the plate stands too. A landscape crop
       cuts the piece off around the waist and leaves the focal point to rescue
       whatever it can. Not 3/4 — that would crop the sides out of the landscape
       originals; 4/5 is the tallest frame the existing photographs survive. */
    aspect-ratio: 4 / 5;
    background: rgba(160, 116, 74, 0.1);
    /* An inner hairline instead of a border: it draws the edge without adding a
       pixel to the box, so the image stays flush in the grid. */
    box-shadow: inset 0 0 0 1px rgba(60, 25, 10, 0.08);
  }

  /* AppImage's own wrapper is a plain block, so it takes the photo's INTRINSIC
     height, not the plate's — the `height: 100%` below then resolves against a
     box that is itself auto-sized, and the image stops short of the frame. Pin
     the wrapper to the plate and the chain has something real to measure. */
  .photo :global(.photo-img) {
    position: absolute;
    inset: 0;
  }

  .photo :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    /* The admin's focal point, so a cropped plate never beheads the figure. */
    object-position: var(--focal, 50% 50%);
    transform: scale(1.01);
    transition: transform 1.4s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .photo:hover :global(img),
  .photo:focus-visible :global(img) {
    transform: scale(1.05);
  }

  /* ── Second angle: pinned exactly over the face photo, cross-fading in on a
     sustained hover. Not a swatch swap — a closer look at the same piece. ── */
  .photo :global(.photo-img-alt) {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.55s ease;
  }

  .photo:hover :global(.photo-img-alt),
  .photo:focus-visible :global(.photo-img-alt) {
    opacity: 1;
  }

  /* Touch has no hover to trigger the reveal — keep only the face photo. */
  @media (hover: none) {
    .photo :global(.photo-img-alt) { display: none; }
  }

  /* ── Raking light ── */
  .photo-rake {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    background: linear-gradient(
      100deg,
      transparent 35%,
      rgba(255, 255, 255, 0.22) 50%,
      transparent 65%
    );
    transform: translateX(-100%);
    transition: transform 1.1s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .photo:hover .photo-rake,
  .photo:focus-visible .photo-rake {
    transform: translateX(100%);
  }

  /* ── The plate: status · year, and the piece's number on the wall ── */
  .plate {
    position: absolute;
    z-index: 3;
    left: 0.75rem;
    right: 0.75rem;
    bottom: 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 0.75rem;
    border-radius: 3px;
    background: rgba(250, 246, 238, 0.88);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    box-shadow: 0 2px 10px rgba(60, 25, 10, 0.14);
    font-size: 0.62rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #7a5035;
    /* It rises with the photograph rather than sitting on it like a sticker. */
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 0.45s ease, transform 0.45s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .photo:hover .plate,
  .photo:focus-visible .plate {
    opacity: 1;
    transform: none;
  }

  /* No hover to reveal it — so it is simply always there. */
  @media (hover: none) {
    .plate { opacity: 1; transform: none; }
  }

  .plate-status { font-weight: 500; }
  .plate-status.status-available { color: #6b8a56; }
  .plate-status.status-reserved { color: #b08820; }
  .plate-status.status-sold { color: #a94438; }
  .plate-status.status-in_progress { color: #7a5035; }

  .plate-roman {
    margin-left: auto;
    font-family: var(--font-display);
    letter-spacing: 0.1em;
    color: #a0745a;
  }

  /* ── Marks: quiet, top-left, in order of prestige ── */
  .marks {
    position: absolute;
    z-index: 3;
    top: 0.75rem;
    left: 0.75rem;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
  }

  .mark {
    padding: 0.28rem 0.5rem;
    border-radius: 2px;
    font-size: 0.55rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    box-shadow: 0 2px 8px rgba(60, 25, 10, 0.18);
  }

  .mark-favorite {
    background: rgba(250, 246, 238, 0.92);
    color: #72300e;
  }

  .mark-new {
    /* Tracks the accent token (was a hardcoded #c0582c): cream-on-copper at the old
       value was 4.17:1 — the same WCAG-AA miss the token itself carried — and this "new"
       wax mark rides every freshly catalogued card on the home reel. The token is now
       #B44E24, which lifts it to 4.80:1. */
    background: var(--color-ember);
    color: #faf6ee;
  }

  @media (prefers-reduced-motion: reduce) {
    .card,
    .photo :global(img),
    .photo:hover :global(img) {
      transition: none;
      transform: none;
    }

    .qbtn.just-saved { animation: none; }
  }

  @media (max-width: 900px) {
    .card,
    .card-flip {
      grid-template-columns: 1fr;
      gap: 0;
    }

    .card .body,
    .card-flip .body {
      order: 2;
      padding: 1.4rem 0.75rem 0.6rem;
    }

    .card .photo,
    .card-flip .photo {
      order: 1;
    }
  }
</style>
