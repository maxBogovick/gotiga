<script lang="ts">
  /**
   * SealedDoor — the closed face of a work whose showing window is shut.
   *
   * When a piece is gated (see showing-window.ts) and the visitor's local clock
   * sits outside its daily window, the card/detail shows this instead of the
   * work: a carved double oak door with a small plaque telling, plainly, when it
   * opens. It is a ritual, not a tease — the schedule is shown openly and the
   * door opens again the same time tomorrow.
   *
   * Two render paths:
   *  - `doorImageUrl` set  → that asset is shown as the door (a painted/photographed
   *    door an editor uploaded), with the plaque laid over it.
   *  - `doorImageUrl` null → the door is drawn in CSS (oak planks, carved panels,
   *    brass hinges + ring handles). No raster asset, unique enough per work, and
   *    it speaks the same engraved/letterpress language as the rest of the cabinet.
   *
   * Purely presentational + static (no looping animation), so a whole gallery of
   * doors stays paint-idle at rest. The caller decides this is closed and must
   * also suppress the card's link + view-transition; this component only paints.
   */
  import { cubicInOut } from 'svelte/easing';
  import { t, lang } from '$lib/i18n';
  import { formatMinutes, windowKind, minutesUntilOpen, type ShowingWindow } from '$lib/showing-window';
  import { houseClock } from '$lib/stores/house-clock.svelte';

  let {
    openFromMin = null,
    openUntilMin = null,
    daysMask = null,
    monthDay = null,
    dateFrom = null,
    dateUntil = null,
    doorImageUrl = null,
    name = '',
    compact = false,
    now = null,
  }: {
    openFromMin?: number | null;
    openUntilMin?: number | null;
    /** Allowed weekdays bitmask (bit0=Mon … bit6=Sun); null → every day. */
    daysMask?: number | null;
    /** "MM-DD" annual date, or one-off "YYYY-MM-DD" range — shown on the plaque. */
    monthDay?: string | null;
    dateFrom?: string | null;
    dateUntil?: string | null;
    /** Optional door asset; when absent the door is drawn procedurally. */
    doorImageUrl?: string | null;
    /** Work name, for the accessible label only. */
    name?: string;
    compact?: boolean;
    /** Moment to read proximity against; defaults to the live house clock. */
    now?: Date | null;
  } = $props();

  let win = $derived<ShowingWindow>({ openFromMin, openUntilMin, daysMask, monthDay, dateFrom, dateUntil });

  // Warmth gathering under the door as opening nears (last hour). 0 → 1.
  const GLOW_HORIZON = 60;
  let glow = $derived.by(() => {
    const m = minutesUntilOpen(win, now ?? houseClock.nowDate, GLOW_HORIZON);
    return m == null ? 0 : 1 - m / GLOW_HORIZON;
  });

  // The opening ritual: when the parent removes the door (its window has opened),
  // this `out` transition parts the leaves and dissolves the door, revealing the
  // work. It drives a single CSS var (--open: 1 closed → 0 open) that the leaves
  // and plaque react to — no per-leaf JS. Reduced-motion degrades to a soft fade.
  function unlatch(_node: Element, { duration = 900 }: { duration?: number } = {}) {
    const reduce =
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reduce) return { duration: 220, css: (t: number) => `opacity:${t}` };
    return {
      duration,
      easing: cubicInOut,
      // For an out transition t runs 1 → 0. Stay opaque while the leaves part,
      // then dissolve.
      css: (t: number) => `--open:${t}; opacity:${Math.min(1, t * 2)};`,
    };
  }
  let kind = $derived(windowKind(win));
  // Hours only matter when they actually restrict the day (from ≠ until).
  let hoursLabel = $derived(
    openFromMin != null && openUntilMin != null && openFromMin !== openUntilMin
      ? `${formatMinutes(openFromMin)}–${formatMinutes(openUntilMin)}`
      : ''
  );
  let phrase = $derived(kind === 'night' ? $t('doorPhraseNight') : $t('doorPhraseDay'));

  let locale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
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
    `${name ? name + ' — ' : ''}${phrase}${dateLabel ? ', ' + dateLabel : ''}${daysLabel ? ', ' + daysLabel : ''}`
  );
</script>

<div
  class="sealed-door"
  class:compact
  class:asset={!!doorImageUrl}
  class:glowing={glow > 0}
  class:glow-near={glow > 0.75}
  style="--glow:{glow.toFixed(3)};"
  role="img"
  aria-label={ariaLabel}
  out:unlatch
>
  {#if doorImageUrl}
    <img class="door-asset" src={doorImageUrl} alt="" />
  {:else}
    <div class="door">
      <div class="leaf left">
        <span class="panel"></span>
        <span class="panel"></span>
        <i class="hinge top"></i>
        <i class="hinge bottom"></i>
        <i class="ring"></i>
      </div>
      <div class="leaf right">
        <span class="panel"></span>
        <span class="panel"></span>
        <i class="hinge top"></i>
        <i class="hinge bottom"></i>
        <i class="ring"></i>
      </div>
      <span class="seam"></span>
    </div>
  {/if}

  <span class="door-glow" aria-hidden="true"></span>

  <span class="plaque">
    <span class="plaque-phrase">{phrase}</span>
    {#if dateLabel}
      <span class="plaque-line">{dateLabel}</span>
    {/if}
    {#if daysLabel}
      <span class="plaque-line">{daysLabel}</span>
    {/if}
    {#if hoursLabel}
      <span class="plaque-hours">{hoursLabel}</span>
    {/if}
  </span>
</div>

<style>
  /* Registered so the unlatch transition interpolates the leaf parting smoothly
     (inherits → the leaves read it). Where @property is unsupported it steps. */
  @property --open {
    syntax: '<number>';
    inherits: true;
    initial-value: 1;
  }

  .sealed-door {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: inherit;
    /* deep, lamplit hall behind the door */
    background:
      radial-gradient(circle at 50% 38%, rgba(58, 38, 24, 0.55), transparent 64%),
      linear-gradient(180deg, #1b120b, #120b06);
  }

  /* ---- procedural oak door ---- */
  .door {
    position: absolute;
    inset: 6%;
    display: grid;
    grid-template-columns: 1fr 1fr;
    border-radius: 5px 5px 3px 3px;
    /* the recessed jamb the leaves sit in */
    box-shadow:
      0 0 0 2px rgba(20, 12, 7, 0.9),
      0 14px 30px rgba(0, 0, 0, 0.5);
  }

  .leaf {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8%;
    padding: 9% 12%;
    /* oak: warm base wash + fine vertical plank grain */
    background:
      repeating-linear-gradient(
        90deg,
        rgba(255, 226, 178, 0.045) 0 2px,
        rgba(0, 0, 0, 0.05) 2px 4px,
        transparent 4px 13px
      ),
      linear-gradient(95deg, #3a2618, #4a3120 38%, #321f12 78%, #281910);
    box-shadow:
      inset 0 0 0 1px rgba(20, 12, 7, 0.7),
      inset 0 1px 0 rgba(255, 220, 170, 0.07);
  }
  .leaf.left {
    border-radius: 4px 0 0 3px;
    /* --open: 1 closed → 0 open (set by the unlatch transition); leaves part. */
    transform: translateX(calc((1 - var(--open, 1)) * -58%));
  }
  .leaf.right {
    border-radius: 0 4px 3px 0;
    transform: translateX(calc((1 - var(--open, 1)) * 58%));
  }

  /* carved recessed panels */
  .panel {
    flex: 1;
    border-radius: 3px;
    background: linear-gradient(160deg, rgba(28, 17, 9, 0.55), rgba(58, 38, 23, 0.25));
    /* bevel: shadowed top-left lip, faint lit bottom-right */
    box-shadow:
      inset 2px 2px 4px rgba(12, 7, 3, 0.85),
      inset -1px -1px 0 rgba(255, 214, 158, 0.08),
      inset 0 0 0 1px rgba(12, 7, 3, 0.4);
  }

  /* the meeting line between the two leaves */
  .seam {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 2px;
    transform: translateX(-50%);
    background: linear-gradient(180deg, rgba(10, 6, 3, 0.2), rgba(10, 6, 3, 0.85), rgba(10, 6, 3, 0.2));
  }

  /* brass hardware */
  .hinge {
    position: absolute;
    width: 13%;
    height: 7%;
    border-radius: 1.5px;
    background: linear-gradient(180deg, #cBA86a, #8a6a3c 60%, #5f4625);
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.55),
      inset 0 1px 0 rgba(255, 240, 200, 0.55);
  }
  .hinge.top {
    top: 11%;
  }
  .hinge.bottom {
    bottom: 11%;
  }
  .leaf.left .hinge {
    left: 0;
  }
  .leaf.right .hinge {
    right: 0;
  }

  /* ring handle near the seam */
  .ring {
    position: absolute;
    top: 47%;
    width: 11%;
    aspect-ratio: 1;
    border-radius: 50%;
    border: 2px solid #9c7842;
    background: radial-gradient(circle at 40% 35%, rgba(255, 240, 200, 0.35), transparent 60%);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.5),
      inset 0 0 0 1px rgba(40, 24, 10, 0.6);
  }
  .leaf.left .ring {
    right: 9%;
  }
  .leaf.right .ring {
    left: 9%;
  }

  /* ---- uploaded door asset ---- */
  .door-asset {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    filter: brightness(0.86) saturate(0.92);
  }

  /* Warmth gathering under the door / through the keyhole as opening nears.
     Opacity is driven by --glow (0..1); paints only when actually glowing. */
  .door-glow {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    opacity: var(--glow, 0);
    background:
      radial-gradient(120% 60% at 50% 100%, rgba(255, 176, 92, 0.55), transparent 62%),
      radial-gradient(40% 26% at 50% 50%, rgba(255, 198, 120, 0.4), transparent 70%);
    mix-blend-mode: screen;
    transition: opacity 1.2s ease;
  }
  .sealed-door:not(.glowing) .door-glow {
    display: none;
  }
  /* A faint flicker only in the final stretch — one element, only on near doors. */
  .glow-near .door-glow {
    animation: door-flicker 3.4s ease-in-out infinite;
  }
  @keyframes door-flicker {
    0%, 100% { opacity: var(--glow, 0); }
    45% { opacity: calc(var(--glow, 0) * 0.82); }
    70% { opacity: calc(var(--glow, 0) * 1.05); }
  }

  /* ---- the plaque ---- */
  .plaque {
    position: relative;
    z-index: 3;
    opacity: var(--open, 1);
    max-width: 82%;
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 11px 16px;
    text-align: center;
    color: #4a3322;
    background: linear-gradient(168deg, #f8f1e7, #ecdcc6);
    border-radius: 3px;
    /* engraved brass-rimmed parchment plate */
    box-shadow:
      0 0 0 1px rgba(111, 59, 36, 0.5),
      0 0 0 3px rgba(180, 142, 86, 0.55),
      0 8px 20px rgba(0, 0, 0, 0.45);
    transform: rotate(-0.6deg);
  }

  .plaque-phrase {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(14px, 1.5vw, 19px);
    font-style: italic;
    line-height: 1.12;
    color: #5a3a23;
  }

  .plaque-hours,
  .plaque-line {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.16em;
    color: #8a5a38;
    font-variant-numeric: tabular-nums;
    text-transform: uppercase;
  }
  .plaque-line {
    letter-spacing: 0.12em;
  }

  .compact .plaque {
    padding: 8px 12px;
  }
  .compact .plaque-phrase {
    font-size: clamp(13px, 3.4vw, 16px);
  }

  @media (prefers-reduced-motion: reduce) {
    .plaque {
      transform: none;
    }
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
