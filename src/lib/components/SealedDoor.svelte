<script lang="ts">
  /**
   * SealedDoor — the closed face of a work whose showing window is shut.
   *
   * When a piece is gated (see showing-window.ts) and the visitor's local clock
   * sits outside its daily window, the card/detail shows this instead of the
   * work: the same affiche language as the house's showings wall
   * (HouseNoticeBoard.svelte "poster" bill) — the work's own image, shaded and
   * unlit, with an engraved plate telling, plainly, when it opens. It is a
   * ritual, not a tease — the schedule is shown openly and the piece lights
   * again the same time tomorrow.
   *
   * Purely presentational + static (no looping animation beyond the near-open
   * flicker), so a whole gallery of sealed cards stays paint-idle at rest. The
   * caller decides this is closed and must also suppress the card's link + view-
   * transition; this component only paints.
   */
  import { cubicInOut } from 'svelte/easing';
  import { t, lang } from '$lib/i18n';
  import { formatMinutes, minutesUntilOpen, openingHeadline, type ShowingWindow } from '$lib/showing-window';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    openFromMin = null,
    openUntilMin = null,
    daysMask = null,
    monthDay = null,
    dateFrom = null,
    dateUntil = null,
    imageUrl = null,
    thumbUrl = null,
    name = '',
    compact = false,
    now = null,
    showSchedule = true,
  }: {
    openFromMin?: number | null;
    openUntilMin?: number | null;
    /** Allowed weekdays bitmask (bit0=Mon … bit6=Sun); null → every day. */
    daysMask?: number | null;
    /** "MM-DD" annual date, or one-off "YYYY-MM-DD" range — shown on the plate. */
    monthDay?: string | null;
    dateFrom?: string | null;
    dateUntil?: string | null;
    /** The work's own image — shaded/unlit until the window opens. */
    imageUrl?: string | null;
    thumbUrl?: string | null;
    /** Work name, for the accessible label only. */
    name?: string;
    compact?: boolean;
    /** Moment to read proximity against; defaults to the live house clock. */
    now?: Date | null;
    /** Set false when the caller already prints the opening time elsewhere
     * next to the card, so the plate isn't a duplicate of the same line. */
    showSchedule?: boolean;
  } = $props();

  let win = $derived<ShowingWindow>({ openFromMin, openUntilMin, daysMask, monthDay, dateFrom, dateUntil });

  // Warmth gathering under the door as opening nears (last hour). 0 → 1.
  const GLOW_HORIZON = 60;
  let glow = $derived.by(() => {
    const m = minutesUntilOpen(win, now ?? houseClock.nowDate, GLOW_HORIZON);
    return m == null ? 0 : 1 - m / GLOW_HORIZON;
  });

  // The opening ritual: when the parent removes this piece (its window has
  // opened), this `out` transition lifts the shade and dissolves the sealed
  // face, revealing the work. It drives a single CSS var (--open: 1 sealed → 0
  // open) that the shade and plate react to — no per-element JS. Reduced-motion
  // degrades to a soft fade.
  function unlatch(_node: Element, { duration = 900 }: { duration?: number } = {}) {
    const reduce =
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reduce) return { duration: 220, css: (t: number) => `opacity:${t}` };
    return {
      duration,
      easing: cubicInOut,
      css: (t: number) => `--open:${t}; opacity:${Math.min(1, t * 2)};`,
    };
  }
  // Hours only matter when they actually restrict the day (from ≠ until).
  let hoursLabel = $derived(
    openFromMin != null && openUntilMin != null && openFromMin !== openUntilMin
      ? `${formatMinutes(openFromMin)}–${formatMinutes(openUntilMin)}`
      : ''
  );

  let locale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
  // The one concrete fact on the plate — relative day + exact clock time.
  let headline = $derived(openingHeadline(win, $t, locale, now ?? houseClock.nowDate));
  // 2024-01-01 was a Monday → index 0..6 maps Mon..Sun (matches the bitmask).
  function dayShort(idx: number): string {
    return new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(new Date(2024, 0, 1 + idx));
  }
  let daysLabel = $derived.by(() => {
    if (daysMask == null) return '';
    const days = daysMask & 0b1111111;
    if (days === 0 || days === 0b1111111) return '';
    const names: string[] = [];
    for (let i = 0; i < 7; i++) if ((days >> i) & 1) names.push(dayShort(i));
    return names.join(' · ');
  });
  function fmtDate(iso: string): string {
    const [y, m, d] = iso.split('-').map(Number);
    // MM-DD (annual) has no year → use a leap year so 02-29 formats.
    const date = iso.length <= 5 ? new Date(2000, (m ?? 1) - 1, d ?? 1) : new Date(y, (m ?? 1) - 1, d ?? 1);
    return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short' }).format(date);
  }
  let dateLabel = $derived.by(() => {
    if (monthDay) return fmtDate(monthDay);
    if (dateFrom && dateUntil) return `${fmtDate(dateFrom)} – ${fmtDate(dateUntil)}`;
    if (dateFrom || dateUntil) return fmtDate((dateFrom || dateUntil) as string);
    return '';
  });

  let ariaLabel = $derived(
    `${name ? name + ' — ' : ''}${headline}${dateLabel ? ', ' + dateLabel : ''}${daysLabel ? ', ' + daysLabel : ''}`
  );
</script>

<div
  class="sealed-door"
  class:compact
  class:glowing={glow > 0}
  class:glow-near={glow > 0.75}
  style="--glow:{glow.toFixed(3)};"
  role="img"
  aria-label={ariaLabel}
  out:unlatch
>
  {#if imageUrl}
    <AppImage src={imageUrl} thumbUrl={thumbUrl} alt="" class="door-art" loading="lazy" />
  {:else}
    <span class="door-fallback" aria-hidden="true">❧</span>
  {/if}

  <!-- a candle picking the work out of the dark, then a heavy shade over it —
       the same light-reveal language as the showings-wall affiche. -->
  <span class="door-spot" aria-hidden="true"></span>
  <span class="door-glow" aria-hidden="true"></span>
  <span class="door-shade" aria-hidden="true"></span>
  <span class="door-grade" aria-hidden="true"></span>
  <span class="door-grain" aria-hidden="true"></span>

  <div class="door-top">
    <span class="door-eyebrow">
      <span class="door-orn" aria-hidden="true">❧</span>
      {$t('posterSoon')}
      <span class="door-eyebrow-rule" aria-hidden="true"></span>
    </span>
    <span class="door-wax" aria-hidden="true"></span>
  </div>

  <div class="door-bottom">
    {#if showSchedule}
    <span class="door-headline">{headline}</span>
    <span class="door-rule" aria-hidden="true"></span>
    {#if hoursLabel}
      <span class="door-hours">{hoursLabel}</span>
    {/if}
    {#if dateLabel}
      <span class="door-line">{dateLabel}</span>
    {/if}
    {#if daysLabel}
      <span class="door-line">{daysLabel}</span>
    {/if}
    {/if}
  </div>
</div>

<style>
  .sealed-door {
    position: absolute;
    inset: 0;
    z-index: 2;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: clamp(12px, 1.6vw, 18px);
    overflow: hidden;
    border-radius: inherit;
    color: #f3e6cf;
    /* deep gallery dark behind the image (and for letterboxing) */
    background: linear-gradient(180deg, #1b120b, #120b06);
  }

  .sealed-door :global(.door-art) {
    position: absolute;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
  }
  .sealed-door :global(.door-art img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
    object-position: center;
    /* sealed: dimmed, unlit silhouette — the work is not yet on view.
       No blur: the photo stays crisp, only shaded (door-shade/door-grade
       overlays below do the "in darkness" work). */
    filter: saturate(0.45) contrast(0.92) brightness(0.92);
  }

  /* no image: a dark engraved sheet with a printed emblem */
  .door-fallback {
    position: absolute;
    inset: 0;
    z-index: 0;
    display: grid;
    place-items: center;
    font-size: clamp(46px, 8vw, 96px);
    color: rgba(201, 160, 99, 0.42);
    background:
      radial-gradient(circle at 50% 38%, rgba(58, 38, 24, 0.5), transparent 64%),
      linear-gradient(180deg, #1b120b, #120b06);
  }

  /* a candle picking the work out of the dark */
  .door-spot {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    background: radial-gradient(
      circle at 50% 42%,
      rgba(255, 214, 150, 0.32),
      rgba(255, 186, 120, 0.12) 26%,
      transparent 52%
    );
    mix-blend-mode: screen;
  }

  /* the lamp grows as the hour nears */
  .door-glow {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    opacity: var(--glow, 0);
    background: radial-gradient(70% 56% at 50% 42%, rgba(255, 176, 92, 0.55), transparent 62%);
    mix-blend-mode: screen;
    transition: opacity 1.2s ease;
  }
  .sealed-door:not(.glowing) .door-glow {
    display: none;
  }
  .glow-near .door-glow {
    animation: door-flicker 3.4s ease-in-out infinite;
  }
  @keyframes door-flicker {
    0%, 100% { opacity: var(--glow, 0); }
    45% { opacity: calc(var(--glow, 0) * 0.82); }
    70% { opacity: calc(var(--glow, 0) * 1.05); }
  }

  /* a heavy shade while still sealed — the work is in darkness until it opens */
  .door-shade {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    opacity: var(--open, 1);
    background: rgba(6, 4, 8, 0.04);
  }

  /* cinematic grade + vignette + scrims that make the overlaid type legible */
  .door-grade {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    background:
      linear-gradient(
        180deg,
        rgba(14, 9, 5, 0.6) 0%,
        rgba(14, 9, 5, 0.12) 20%,
        transparent 38%,
        rgba(14, 9, 5, 0.5) 66%,
        rgba(10, 6, 3, 0.93) 100%
      ),
      radial-gradient(125% 92% at 50% 36%, transparent 42%, rgba(10, 6, 3, 0.5) 100%);
  }

  /* faint print grain over everything */
  .door-grain {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
    opacity: 0.5;
    background:
      radial-gradient(circle at 82% 16%, rgba(150, 105, 55, 0.08), transparent 26%),
      radial-gradient(circle at 16% 86%, rgba(150, 105, 55, 0.06), transparent 24%);
  }

  /* top + bottom content bands sit above all overlays */
  .door-top,
  .door-bottom {
    position: relative;
    z-index: 4;
    opacity: var(--open, 1);
  }
  .door-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }
  .door-bottom {
    display: grid;
    gap: 6px;
    text-align: center;
    justify-items: center;
  }

  .door-eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: clamp(9px, 0.7vw, 11px);
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: #f0b48f;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
  }
  .door-orn {
    flex-shrink: 0;
    font-size: 12px;
    line-height: 1;
  }
  .door-eyebrow-rule {
    flex: 1;
    min-width: 12px;
    height: 1px;
    background: linear-gradient(90deg, rgba(240, 180, 143, 0.6), transparent);
  }

  /* a wax seal pressed on the still-sealed face */
  .door-wax {
    flex-shrink: 0;
    position: relative;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 32%, #d2683f, #8a2f1c 62%, #5f2012);
    box-shadow:
      0 1px 5px rgba(10, 4, 2, 0.6),
      inset 0 1px 0 rgba(255, 200, 170, 0.45);
  }
  .door-wax::after {
    content: '❧';
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 11px;
    color: rgba(40, 12, 6, 0.55);
  }

  /* the one concrete fact — bold and warm, meant to be read at a glance */
  .door-headline {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-weight: 600;
    font-size: clamp(17px, 1.9vw, 23px);
    line-height: 1.12;
    color: #ffcb96;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.75);
  }

  .door-rule {
    width: 38%;
    min-width: 40px;
    height: 0;
    border-top: 1px solid rgba(255, 228, 196, 0.32);
  }

  .door-line,
  .door-hours {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #ecbd95;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
  }

  .compact .door-headline {
    font-size: clamp(14px, 3.4vw, 17px);
  }

  @media (prefers-reduced-motion: reduce) {
    .glow-near .door-glow {
      animation: none;
    }
  }

  @media (pointer: coarse) {
    .glow-near .door-glow {
      animation: none;
    }
  }
</style>
