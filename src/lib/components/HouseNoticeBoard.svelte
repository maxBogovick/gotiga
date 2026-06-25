<script lang="ts">
  /**
   * HouseNoticeBoard — the museum's wall of exhibition bills ("Афиша").
   *
   * A wall of pasted portrait affiches, just below the hero, announcing the few
   * showings the museum is about to open. Each bill is a real poster: the work's
   * face full-bleed, with the type laid OVER it. Because the works are shot on
   * gothic black, a *darkening* veil would make them vanish — so instead a warm
   * candle-spot picks the work out of the dark (a light reveal), a thin cool shade
   * marks "not yet lit", and the lamp grows (reused proximity glow) as the hour
   * nears. A wax seal sits on a sealed bill; the moment the show opens the shade
   * lifts, the image brightens, and a "ОТКРЫТО" overprint stamps the corner. Over a
   * bottom scrim: an eyebrow (СКОРО / ОТКРЫТО, fleuron ❧ work · ❦ room), the big
   * display name, a rule, and *when* it opens stated atmospherically ("after dark",
   * "in daylight", a weekday, a date) — never a countdown. The nearest opening is
   * the featured affiche; the rest are smaller bills pasted askew. A room borrows a
   * representative work's face; a work with no image falls back to an engraved
   * emblem. Curiosity, not anxiety — the stamp is a state, not a deadline.
   *
   * No backend of its own: rooms come from the showing-rooms store (→ /hall/[id]),
   * standalone gated works from the home page's figurine list (→ /figurines/[id]).
   * A bill earns its place only when gated and open now or opening inside the
   * horizon. Nothing here when the programme is empty — the whole wall hides.
   *
   * Ethics (see SHOWING_WINDOW_ROADMAP.md): name + when, not the works behind the
   * door; a 7-day horizon, not ambient noise; no clock, no "hurry". The "ОТКРЫТО"
   * stamp is a state, not a deadline. Curiosity, not anxiety.
   */
  import { onMount, tick } from 'svelte';
  import { turnSound } from '$lib/stores/page-turn-sound.svelte';
  import { playTurnSound } from '$lib/audio/page-turn-sounds';

  import { t, lang } from '$lib/i18n';
  import type { FigurineListItem, ProgrammeSettings } from '$lib/types/api';
  import AppImage from '$lib/components/AppImage.svelte';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { api } from '$lib/api';
  import {
    roomToWindow,
    isShowingOpen,
    isGated,
    minutesUntilOpen,
    describeOpening,
    openingWhenLabel,
    type ShowingWindow,
  } from '$lib/showing-window';

  let { figurines = [] }: { figurines?: FigurineListItem[] } = $props();

  const HORIZON_DAYS = 7;
  const GLOW_HORIZON = 60;
  const SECONDARY_PER_PAGE = 2;

  // ── Pagination state (derived values declared after `notes` below) ──────────
  let secondaryPage = $state(0);
  let flipDir = $state(1); // 1 = forward, -1 = backward

  function toRoman(n: number): string {
    if (n <= 0) return 'I';
    const vals = [10, 9, 5, 4, 1];
    const syms = ['X', 'IX', 'V', 'IV', 'I'];
    let r = '';
    for (let i = 0; i < vals.length; i++) {
      while (n >= vals[i]) { r += syms[i]; n -= vals[i]; }
    }
    return r;
  }

  let programmeSettings = $state<ProgrammeSettings | null>(null);

  const maxNotes = $derived(
    programmeSettings && programmeSettings.maxShowings > 0 ? programmeSettings.maxShowings : 0
  );
  const caseBg = $derived(programmeSettings?.caseBg?.trim() || null);

  // ── Cast-bronze molding, derived from a single base tone ────────────────────
  // One light→dark→light sweep reads as a beveled top-lit metal profile.
  function shade(hex: string, amount: number): string {
    const h = hex.replace('#', '');
    const n = h.length === 3 ? h.split('').map((c) => c + c).join('') : h;
    const r = parseInt(n.slice(0, 2), 16);
    const g = parseInt(n.slice(2, 4), 16);
    const b = parseInt(n.slice(4, 6), 16);
    const adj = (v: number) =>
      Math.max(0, Math.min(255, Math.round(amount >= 0 ? v + (255 - v) * amount : v * (1 + amount))));
    return `rgb(${adj(r)}, ${adj(g)}, ${adj(b)})`;
  }
  const HEX_RE = /^#?[0-9a-fA-F]{3}([0-9a-fA-F]{3})?$/;
  const frameMode = $derived(programmeSettings?.frameMode || 'gradient');
  const frameGradient = $derived.by(() => {
    const tone = programmeSettings?.frameTone?.trim();
    const valid = tone && HEX_RE.test(tone) ? tone : null;
    if (frameMode === 'flat') {
      // a solid molding tone, no bevel sweep
      return valid ? shade(valid, -0.18) : '#8a6738';
    }
    if (!valid) return null; // gradient mode, no tone → CSS fallback (built-in bronze)
    return `linear-gradient(146deg, ${shade(valid, 0.42)} 0%, ${shade(valid, 0.14)} 14%, ${shade(valid, -0.18)} 32%, ${shade(valid, -0.58)} 52%, ${shade(valid, -0.74)} 64%, ${shade(valid, -0.34)} 82%, ${shade(valid, 0.3)} 100%)`;
  });
  const frameThickness = $derived(
    programmeSettings?.frameThickness != null && programmeSettings.frameThickness > 0
      ? `${programmeSettings.frameThickness}px`
      : null
  );
  const caseStyle = $derived(
    [
      caseBg ? `background: ${caseBg}` : '',
      frameGradient ? `--frame-gradient: ${frameGradient}` : '',
      frameThickness ? `--frame-thickness: ${frameThickness}` : '',
    ]
      .filter(Boolean)
      .join('; ')
  );
  const curatorNote = $derived.by(() => {
    const s = programmeSettings;
    if (!s) return null;
    return $lang === 'ru'
      ? (s.curatorNoteRu?.trim() || s.curatorNoteEn?.trim() || null)
      : (s.curatorNoteEn?.trim() || null);
  });
  const curatorSign = $derived.by(() => {
    const s = programmeSettings;
    if (!s) return null;
    return $lang === 'ru'
      ? (s.curatorSignRu?.trim() || s.curatorSignEn?.trim() || null)
      : (s.curatorSignEn?.trim() || null);
  });

  let locale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');

  onMount(() => {
    houseClock.start();
    showingRooms.load();
    turnSound.load();
    api.getProgrammeSettings().then((s) => { programmeSettings = s; }).catch(() => {});
  });

  /** The keyhole-veil art behind a bill: the work's face, shown veiled until open. */
  interface Art {
    faceImageUrl: string | null;
    thumbUrl: string | null;
    focalX: number | null;
    focalY: number | null;
    revealRadius: number | null;
    darkness: number | null;
  }

  interface Note {
    key: string;
    name: string;
    href: string;
    isRoom: boolean;
    openNow: boolean;
    when: string;
    glow: number;
    art: Art;
    sortKey: number;
  }

  const toArt = (f: FigurineListItem): Art => ({
    faceImageUrl: f.faceImageUrl ?? null,
    thumbUrl: f.thumbUrl ?? null,
    focalX: f.focalX ?? null,
    focalY: f.focalY ?? null,
    revealRadius: f.revealRadius ?? null,
    darkness: f.darkness ?? null,
  });

  // A bill for anything the museum is about to (or has just) open, drawn from two
  // sources: showing *rooms* that hold works (→ the hall), and standalone *works*
  // with their own hours and no room (→ the work). A bill earns its place when
  // gated and open now or opening within the horizon. Each carries the work's face
  // (a room borrows a representative work's), shown veiled until the show opens.
  // Recomputed off the house clock so a bill warms, reveals, and the wall
  // appears/vanishes live.
  let notes = $derived.by(() => {
    const now = houseClock.nowDate;

    const build = (
      key: string,
      name: string,
      href: string,
      isRoom: boolean,
      win: ShowingWindow,
      art: Art
    ): Note | null => {
      if (!isGated(win)) return null;
      const openNow = isShowingOpen(win, now);
      const desc = openNow ? null : describeOpening(win, now, HORIZON_DAYS);
      if (!openNow && !desc) return null;
      const mins = minutesUntilOpen(win, now, GLOW_HORIZON);
      return {
        key,
        name,
        href,
        isRoom,
        openNow,
        when: desc ? openingWhenLabel(desc, $t, locale) : '',
        glow: mins == null ? 0 : 1 - mins / GLOW_HORIZON,
        art,
        sortKey: openNow ? -Infinity : desc!.at.getTime(),
      };
    };

    // Rooms — only those holding works; borrow a representative work's face
    // (prefer one that actually has an image) for the veil panel.
    const counts = new Map<string, number>();
    const repr = new Map<string, FigurineListItem>();
    for (const f of figurines) {
      if (!f.showingRoomId) continue;
      counts.set(f.showingRoomId, (counts.get(f.showingRoomId) ?? 0) + 1);
      const cur = repr.get(f.showingRoomId);
      if (!cur || (!cur.faceImageUrl && f.faceImageUrl)) repr.set(f.showingRoomId, f);
    }
    const roomNotes = showingRooms.list
      .filter((room) => (counts.get(room.id) ?? 0) > 0)
      .map((room) => {
        const rep = repr.get(room.id);
        return build(
          `room:${room.id}`,
          room.name,
          `/hall/${room.id}`,
          true,
          roomToWindow(room),
          rep ? toArt(rep) : { faceImageUrl: null, thumbUrl: null, focalX: null, focalY: null, revealRadius: null, darkness: null }
        );
      });

    // Standalone gated works — their own hours, no room.
    const workNotes = figurines
      .filter((f) => !f.showingRoomId && f.status !== 'in_progress')
      .map((f) =>
        build(
          `work:${f.id}`,
          f.name,
          `/figurines/${f.id}`,
          false,
          { openFromMin: f.openFromMin, openUntilMin: f.openUntilMin },
          toArt(f)
        )
      );

    return [...roomNotes, ...workNotes]
      .filter((n): n is Note => n !== null)
      .sort((a, b) => a.sortKey - b.sortKey)
      .slice(0, maxNotes > 0 ? maxNotes : undefined);
  });

  // ── Pagination derived (after notes) ────────────────────────────────────────
  const secondaryAll = $derived(notes.slice(1));
  const totalSecondaryPages = $derived(
    Math.max(1, Math.ceil(secondaryAll.length / SECONDARY_PER_PAGE))
  );
  const visibleSecondary = $derived(
    secondaryAll.slice(
      secondaryPage * SECONDARY_PER_PAGE,
      (secondaryPage + 1) * SECONDARY_PER_PAGE
    )
  );
  const hasMultiplePages = $derived(totalSecondaryPages > 1);
  const nextPage = $derived((secondaryPage + 1) % totalSecondaryPages);
  const prevPage = $derived((secondaryPage - 1 + totalSecondaryPages) % totalSecondaryPages);
  const pageIndices = $derived<number[]>(Array.from({ length: totalSecondaryPages }, (_, i) => i));

  $effect(() => {
    if (secondaryPage >= totalSecondaryPages) secondaryPage = 0;
  });

  // ── Fade Through Dark → Daguerreotype develop ───────────────────────────────
  // Phase 1 (220ms): cards dissolve into darkness — opacity + brightness → 0.
  // Silence. Content swaps while invisible.
  // Phase 2 (520ms): new cards develop like a daguerreotype: start as a pale,
  // high-contrast, desaturated ghost and slowly resolve into full tone and colour.
  let cardsEl = $state<HTMLElement | null>(null);
  let displayedCards = $state<Note[]>([]);
  let isFlipping = $state(false);

  $effect(() => {
    if (!isFlipping) displayedCards = [...visibleSecondary];
  });

  const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

  async function turnPage(dir: 1 | -1 = 1, target?: number) {
    if (isFlipping || !cardsEl) return;
    isFlipping = true;
    flipDir = dir;

    const el = cardsEl;
    const sound = turnSound.value;
    if (sound !== 'off') playTurnSound(sound, dir === 1 ? 'forward' : 'backward');

    // ── Phase 1: fade to dark ────────────────────────────────────────────────
    el.style.transition = 'opacity 1000ms ease-in, filter 1000ms ease-in';
    el.style.opacity = '0';
    el.style.filter = 'brightness(0)';
    await sleep(600);

    // ── Swap content while invisible ─────────────────────────────────────────
    secondaryPage = target ?? (dir === 1 ? nextPage : prevPage);
    displayedCards = [...visibleSecondary];
    await tick();

    // ── Phase 2: daguerreotype develop ──────────────────────────────────────
    // Start: pale ghost — high contrast, desaturated, slightly blurred, bright
    el.style.transition = 'none';
    el.style.opacity = '1';
    el.style.filter = 'brightness(2.4) contrast(3.2) saturate(0) blur(2.5px)';
    void el.offsetHeight; // flush

    el.style.transition =
      'opacity 480ms ease-out, filter 1000ms cubic-bezier(0.22, 1, 0.36, 1)';
    el.style.opacity = '1';
    el.style.filter = 'brightness(1) contrast(1) saturate(1) blur(0px)';
    await sleep(1000);

    el.style.cssText = '';
    isFlipping = false;
  }
</script>

<!-- one pasted affiche -->
{#snippet bill(note: Note, featured: boolean)}
  <a
    class="poster"
    class:featured={featured}
    class:is-open={note.openNow}
    class:glowing={note.glow > 0}
    style="--glow:{note.glow.toFixed(3)}; --fx:{((note.art.focalX ?? 0.5) * 100).toFixed(1)}%; --fy:{((note.art.focalY ?? 0.42) * 100).toFixed(1)}%"
    href={note.href}
    aria-label="{note.name} — {note.openNow ? $t('boardNoteOpenNow') : note.when}"
  >
    <!-- full-bleed image: the work itself is the poster -->
    {#if note.art.faceImageUrl}
      <AppImage
        src={note.art.faceImageUrl}
        thumbUrl={note.art.thumbUrl}
        alt=""
        class="poster-art"
        loading="lazy"
      />
    {:else}
      <span class="poster-fallback" aria-hidden="true">{note.isRoom ? '❦' : '❧'}</span>
    {/if}

    <!-- light, not dark: a candle picks the work out of the gothic black;
         the lamp grows as the hour nears. A thin shade marks "not yet lit". -->
    <span class="poster-spot" aria-hidden="true"></span>
    <span class="poster-glow" aria-hidden="true"></span>
    {#if !note.openNow}
      <span class="poster-shade" aria-hidden="true"></span>
    {/if}
    <span class="poster-grade" aria-hidden="true"></span>
    <span class="poster-grain" aria-hidden="true"></span>

    <div class="poster-top">
      <span class="poster-eyebrow">
        <span class="poster-orn" aria-hidden="true">{note.isRoom ? '❦' : '❧'}</span>
        {note.openNow ? $t('posterOnView') : $t('posterSoon')}
        <span class="poster-eyebrow-rule" aria-hidden="true"></span>
      </span>
      {#if note.openNow}
        <span class="poster-stamp" aria-hidden="true">{$t('posterStampOpen')}</span>
      {:else}
        <span class="poster-wax" aria-hidden="true"></span>
      {/if}
    </div>

    <div class="poster-bottom">
      <h3 class="poster-name">{note.name}</h3>
      <span class="poster-rule" aria-hidden="true"></span>
      <span class="poster-date">{note.openNow ? $t('hallOpenNow') : note.when}</span>
      <span class="poster-enter">
        {$t('boardNoteEnter')}
        <svg width="15" height="8" viewBox="0 0 15 8" fill="none" aria-hidden="true">
          <path d="M0 4H14M14 4L10.5 1M14 4L10.5 7" stroke="currentColor" stroke-width="1" />
        </svg>
      </span>
    </div>

    <span class="poster-peel" aria-hidden="true"></span>
  </a>
{/snippet}

{#if notes.length > 0}
  <section class="wall" aria-labelledby="wall-title">
    <div class="programme">
      <!-- A glazed programme case set into the parchment wall: one large nearest
           affiche, the rest pinned smaller; a recessed frame + glass sheen + screws
           make it a discrete museum object, not "more cards". -->
      <div class="case" class:no-frame={frameMode === 'none'} style={caseStyle}>
        <span class="case-screw tl" aria-hidden="true"></span>
        <span class="case-screw tr" aria-hidden="true"></span>
        <span class="case-screw bl" aria-hidden="true"></span>
        <span class="case-screw br" aria-hidden="true"></span>
        <div class="case-plate">
          <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('boardEyebrow')}</p>
          <h2 id="wall-title" class="wall-title">{$t('boardTitle')}</h2>
        </div>
        <div class="case-inner" class:solo={notes.length === 1}>
          <div class="case-feature">
            {@render bill(notes[0], true)}
          </div>
          {#if secondaryAll.length > 0}
            <div class="case-rest" class:has-pages={hasMultiplePages}>

              <div class="case-rest-track">
                <div class="case-rest-cards" bind:this={cardsEl}>
                  {#each displayedCards as note (note.key)}
                    {@render bill(note, false)}
                  {/each}
                </div>
              </div>

              {#if hasMultiplePages}
                <!-- Bottom-centre navigation: ← [I · III] → -->
                <div class="page-nav">
                  <button
                    class="page-nav-btn"
                    onclick={() => turnPage(-1)}
                    aria-label="Previous page"
                  >
                    <svg width="18" height="8" viewBox="0 0 18 8" fill="none" aria-hidden="true">
                      <path d="M18 4H1M1 4L5 1M1 4L5 7" stroke="currentColor" stroke-width="1"/>
                    </svg>
                  </button>

                  <div class="page-counter" aria-live="polite" aria-atomic="true">
                    {#each pageIndices as idx}
                      <button
                        class="page-dot"
                        class:active={idx === secondaryPage}
                        onclick={() => { if (idx !== secondaryPage) turnPage(idx > secondaryPage ? 1 : -1, idx); }}
                        aria-label="Page {toRoman(idx + 1)}"
                      ></button>
                    {/each}
                    <span class="page-num">{toRoman(secondaryPage + 1)}</span>
                  </div>

                  <button
                    class="page-nav-btn"
                    onclick={() => turnPage(1)}
                    aria-label="Next page"
                  >
                    <svg width="18" height="8" viewBox="0 0 18 8" fill="none" aria-hidden="true">
                      <path d="M0 4H17M17 4L13 1M17 4L13 7" stroke="currentColor" stroke-width="1"/>
                    </svg>
                  </button>
                </div>
              {/if}
            </div>
          {/if}
          <span class="case-glass" aria-hidden="true"></span>
        </div>
      </div>

      <!-- the keeper's note: lets the open field beside the case carry the
           museum's voice instead of sitting empty. A state of calm, not a CTA. -->
      <aside class="curator">
        <span class="curator-orn" aria-hidden="true">❦</span>
        <p class="curator-note">{curatorNote ?? $t('boardCuratorNote')}</p>
        <p class="curator-sign">{curatorSign ?? $t('boardCuratorSign')}</p>
      </aside>
    </div>
  </section>
{/if}

<style>
  .wall {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(8px, 1.4vw, 18px) clamp(20px, 4.5vw, 64px) clamp(22px, 3vw, 40px);
  }

  /* case on the left, the keeper's note set in the open field to its right */
  .programme {
    display: flex;
    align-items: center;
    gap: clamp(28px, 5vw, 84px);
  }
  .programme .case {
    flex: 0 1 auto;
  }

  .curator {
    flex: 1 1 220px;
    max-width: 360px;
    align-self: center;
  }
  .curator-orn {
    display: block;
    margin-bottom: 14px;
    font-size: 22px;
    line-height: 1;
    color: var(--color-ember, #c65f3c);
    opacity: 0.75;
  }
  .curator-note {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: clamp(18px, 1.5vw, 24px);
    line-height: 1.5;
    color: var(--color-deep, #6f3b24);
  }
  .curator-sign {
    margin: 16px 0 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  /* on narrow screens the note drops below the case rather than squeezing it */
  @media (max-width: 900px) {
    .programme {
      flex-direction: column;
      align-items: flex-start;
      gap: clamp(20px, 4vw, 32px);
    }
    .curator {
      max-width: 540px;
    }
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  .eyebrow-rule {
    display: inline-block;
    width: 26px;
    height: 1px;
    background: var(--color-ember);
    opacity: 0.65;
    flex-shrink: 0;
  }

  .wall-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(26px, 2.4vw, 38px);
    font-weight: 300;
    line-height: 0.98;
    color: var(--color-ink-primary);
  }

  /* a portrait affiche: the work fills the sheet, the type sits over it.
     Fixed, modest width (never grows) — a few bills pinned to a wall. */
  .poster {
    position: relative;
    isolation: isolate;
    flex: 0 0 clamp(230px, 24vw, 300px);
    max-width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    aspect-ratio: 3 / 4;
    padding: clamp(14px, 1.5vw, 20px);
    overflow: hidden;
    text-decoration: none;
    color: #f3e6cf;
    border: 1px solid rgba(18, 11, 6, 0.6);
    border-radius: 3px;
    /* deep gallery dark behind the image (and for letterboxing) */
    background: linear-gradient(180deg, #1b120b, #120b06);
    box-shadow:
      0 16px 38px rgba(30, 17, 8, 0.3),
      0 2px 0 rgba(30, 17, 8, 0.16);
    transform: rotate(-0.5deg);
    transition: transform 0.28s ease, box-shadow 0.28s ease;
  }
  .poster:nth-child(2n) { transform: rotate(0.6deg); }
  .poster:nth-child(3n) { transform: rotate(-0.3deg); }

  .poster:hover {
    transform: rotate(0deg) translateY(-4px);
    box-shadow:
      0 24px 50px rgba(30, 17, 8, 0.4),
      0 2px 0 rgba(30, 17, 8, 0.16);
  }

  /* full-bleed work image */
  .poster :global(.poster-art) {
    position: absolute;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
  }
  .poster :global(.poster-art img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
    object-position: center;
    /* sealed: blurred silhouette — work is behind the door */
    filter: saturate(0.3) contrast(0.9) brightness(0.7) blur(5px);
    transition: filter 0.7s ease, transform 0.7s ease;
  }
  .poster.is-open :global(.poster-art img) {
    filter: saturate(1) contrast(1.06) brightness(1.12) blur(0px);
  }
  .poster:hover :global(.poster-art img) {
    transform: scale(1.04);
  }
  .poster.is-open:hover :global(.poster-art img) {
    filter: saturate(1.02) contrast(1.07) brightness(1.14) blur(0px);
  }

  /* no image: a dark engraved sheet with a printed emblem */
  .poster-fallback {
    position: absolute;
    inset: 0;
    z-index: 0;
    display: grid;
    place-items: center;
    font-size: clamp(60px, 9vw, 130px);
    color: rgba(201, 160, 99, 0.42);
    background:
      radial-gradient(circle at 50% 38%, rgba(58, 38, 24, 0.5), transparent 64%),
      linear-gradient(180deg, #1b120b, #120b06);
  }

  /* a candle picking the work out of the dark (light reveal, not a black veil) */
  .poster-spot {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    background: radial-gradient(
      circle at var(--fx, 50%) var(--fy, 42%),
      rgba(255, 214, 150, 0.32),
      rgba(255, 186, 120, 0.12) 26%,
      transparent 52%
    );
    mix-blend-mode: screen;
  }
  .poster.is-open .poster-spot {
    opacity: 0.45;
  }

  /* the lamp grows as the hour nears */
  .poster-glow {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
    opacity: var(--glow, 0);
    background: radial-gradient(
      70% 56% at var(--fx, 50%) var(--fy, 42%),
      rgba(255, 176, 92, 0.55),
      transparent 62%
    );
    mix-blend-mode: screen;
    transition: opacity 1.2s ease;
  }
  .poster:not(.glowing) .poster-glow {
    display: none;
  }

  /* a heavy shade while still sealed — work is in darkness until the door opens */
  .poster-shade {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    background: rgba(6, 4, 8, 0.72);
    transition: background 0.6s ease;
  }
  .poster.is-open .poster-shade {
    background: rgba(6, 4, 8, 0.0);
  }

  /* cinematic grade + vignette + scrims that make the overlaid type legible */
  .poster-grade {
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
  .poster-grain {
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
  .poster-top,
  .poster-bottom {
    position: relative;
    z-index: 4;
  }
  .poster-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }
  .poster-bottom {
    display: grid;
    gap: 7px;
  }

  .poster-eyebrow {
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
  .poster-orn {
    flex-shrink: 0;
    font-size: 12px;
    line-height: 1;
  }
  .poster-eyebrow-rule {
    flex: 1;
    min-width: 12px;
    height: 1px;
    background: linear-gradient(90deg, rgba(240, 180, 143, 0.6), transparent);
  }

  .poster-name {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-weight: 600;
    font-size: clamp(21px, 1.9vw, 29px);
    line-height: 1.02;
    color: #fff4e2;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.75);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .poster.featured .poster-name {
    font-size: clamp(26px, 2.6vw, 40px);
  }

  .poster-rule {
    width: 38%;
    min-width: 56px;
    height: 0;
    border-top: 1px solid rgba(255, 228, 196, 0.32);
  }
  .poster-date {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: clamp(15px, 1.35vw, 21px);
    line-height: 1.06;
    color: #ecbd95;
    text-shadow: 0 1px 6px rgba(0, 0, 0, 0.7);
  }
  .poster.featured .poster-date {
    font-size: clamp(17px, 1.6vw, 25px);
  }

  .poster-enter {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: clamp(9px, 0.72vw, 11px);
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(255, 240, 225, 0.85);
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
  }
  .poster-enter svg {
    transition: transform 0.2s ease;
  }
  .poster:hover .poster-enter {
    color: #fff;
  }
  .poster:hover .poster-enter svg {
    transform: translateX(3px);
  }

  /* a wax seal pressed on the still-sealed bill */
  .poster-wax {
    flex-shrink: 0;
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 32%, #d2683f, #8a2f1c 62%, #5f2012);
    box-shadow:
      0 1px 5px rgba(10, 4, 2, 0.6),
      inset 0 1px 0 rgba(255, 200, 170, 0.45);
  }
  .poster-wax::after {
    content: '❧';
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 12px;
    color: rgba(40, 12, 6, 0.55);
  }

  /* "ОТКРЫТО" overprint — a NOW-ON-VIEW rubber stamp (a STATE, not a clock) */
  .poster-stamp {
    flex-shrink: 0;
    padding: 5px 11px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: clamp(10px, 0.9vw, 14px);
    font-weight: 800;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #ffd9b0;
    border: 2px solid rgba(255, 180, 120, 0.85);
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px rgba(255, 180, 120, 0.3);
    transform: rotate(6deg);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  /* a quietly dog-eared corner (the lit back of the sheet) that lifts on hover */
  .poster-peel {
    position: absolute;
    right: 0;
    bottom: 0;
    z-index: 5;
    width: 20px;
    height: 20px;
    pointer-events: none;
    background: linear-gradient(135deg, transparent 50%, rgba(244, 230, 207, 0.16) 50%);
    transition: width 0.28s ease, height 0.28s ease;
  }
  .poster:hover .poster-peel {
    width: 28px;
    height: 28px;
  }

  .poster.is-open .poster-date {
    color: #f3a878;
  }

  /* ── the glazed programme case ──────────────────────────────────── */
  /* a discrete object set INTO the parchment wall (recessed frame, glass sheen,
     mounting screws) so it reads as an announcement case, not more content. */
  .case {
    position: relative;
    max-width: clamp(560px, 64vw, 860px);
    /* extra padding makes room for the cast-bronze molding ring (::before) */
    padding: clamp(22px, 2.6vw, 38px);
    border: 1px solid #120c05;
    border-radius: 8px;
    background:
      radial-gradient(ellipse at 30% 15%, rgba(160, 110, 50, 0.12) 0%, transparent 55%),
      radial-gradient(ellipse at 75% 85%, rgba(80, 45, 15, 0.18) 0%, transparent 50%),
      linear-gradient(170deg, #2e2014 0%, #241a0e 55%, #1c1208 100%);
    box-shadow:
      /* the glass sits recessed inside the molding — a deep inner groove */
      inset 0 0 22px rgba(0, 0, 0, 0.6),
      inset 0 2px 5px rgba(0, 0, 0, 0.55),
      inset 0 0 0 1px rgba(0, 0, 0, 0.45),
      /* the whole case lifts off the parchment wall */
      0 10px 34px rgba(20, 10, 4, 0.42),
      0 2px 6px rgba(20, 10, 4, 0.3),
      0 1px 0 rgba(255, 248, 232, 0.22);
  }

  /* ── cast-bronze molding ring ─────────────────────────────────────
     A single gradient swept across the frame reads as one beveled,
     top-lit metal profile: gilt catches light at the upper edge, sinks
     to shadowed bronze at the lower-right, with a bright return at the
     corners. Painted as a ring via mask so only the border band shows. */
  .case::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 8px;
    /* molding thickness — admin-overridable via --frame-thickness */
    padding: var(--frame-thickness, clamp(11px, 1.3vw, 17px));
    /* bronze sweep — admin-overridable via --frame-gradient */
    background: var(
      --frame-gradient,
      linear-gradient(
        146deg,
        #e9d199 0%,
        #c39d59 14%,
        #8a6738 32%,
        #4a341c 52%,
        #2c1d0f 64%,
        #6e4f2a 82%,
        #cba869 100%
      )
    );
    -webkit-mask:
      linear-gradient(#000 0 0) content-box,
      linear-gradient(#000 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    box-shadow:
      /* crisp gilt catch on the outer top edge */
      inset 0 1px 0 rgba(255, 240, 200, 0.55),
      /* shadowed lower return */
      inset 0 -1px 0 rgba(0, 0, 0, 0.55),
      /* the bevel rolling down into the glass groove */
      inset 0 0 0 1px rgba(0, 0, 0, 0.35);
    pointer-events: none;
  }
  /* frame disabled: no molding ring, a plain hairline edge instead */
  .case.no-frame::before {
    display: none;
  }
  .case.no-frame {
    padding: clamp(16px, 2vw, 30px);
    border-color: rgba(120, 86, 44, 0.4);
  }
  .case-plate {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin-bottom: clamp(14px, 1.6vw, 22px);
    padding-bottom: 12px;
    border-bottom: 1px solid rgba(180, 130, 60, 0.25);
  }
  .case-plate .eyebrow {
    margin-bottom: 0;
    color: #8a6a48;
  }
  .case-plate .wall-title {
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
    color: #d4b896;
  }
  .case-inner {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: clamp(14px, 1.6vw, 26px);
    align-items: start;
  }
  .case-inner.solo {
    grid-template-columns: auto;
    justify-content: start;
  }
  .case-feature {
    width: clamp(220px, 26vw, 360px);
  }
  .case-feature .poster {
    flex: none;
    width: 100%;
    transform: rotate(-0.4deg);
  }
  /* ── Secondary column with pagination ───────────────────────────── */
  .case-rest {
    position: relative;
    display: flex;
    flex-direction: column;
    align-self: stretch;
    min-width: 0;
    gap: clamp(10px, 1vw, 16px);
  }
  .case-rest.has-pages {
    padding-bottom: 4px;
  }

  .case-rest-track {
    flex: 1;
    min-width: 0;
    min-height: clamp(180px, 22vw, 300px);
  }

  .case-rest-cards {
    display: flex;
    align-items: flex-start;
    gap: clamp(12px, 1.3vw, 20px);
    width: 100%;
    height: 100%;
    will-change: opacity, filter;
  }
  .case-rest-cards .poster {
    flex: 1 1 0;
    width: auto;
  }
  .case-rest-cards .poster:nth-child(1) { transform: rotate(-0.3deg); }
  .case-rest-cards .poster:nth-child(2) { transform: rotate(0.7deg); }

  /* ── Bottom navigation bar ───────────────────────────────────────── */
  .page-nav {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding-top: 2px;
    border-top: 1px solid rgba(180, 130, 60, 0.18);
  }

  .page-nav-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 28px;
    padding: 0;
    border: 1px solid rgba(200, 165, 110, 0.32);
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.04);
    color: rgba(212, 184, 150, 0.75);
    cursor: pointer;
    transition: border-color 0.18s, color 0.18s, background 0.18s;
  }
  .page-nav-btn:hover {
    border-color: rgba(200, 165, 110, 0.7);
    color: rgba(230, 205, 165, 1);
    background: rgba(255, 255, 255, 0.09);
  }
  .page-nav-btn:active {
    background: rgba(255, 255, 255, 0.14);
  }

  .page-counter {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .page-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    border: none;
    padding: 0;
    cursor: pointer;
    background: rgba(200, 165, 110, 0.25);
    transition: background 0.22s ease, transform 0.22s ease;
  }
  .page-dot.active {
    background: rgba(200, 165, 110, 0.9);
    transform: scale(1.4);
  }

  .page-num {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-style: italic;
    color: rgba(200, 165, 110, 0.7);
    min-width: 18px;
    text-align: center;
    letter-spacing: 0.04em;
  }
  /* a faint diagonal glass highlight across the whole case */
  .case-glass {
    position: absolute;
    inset: 0;
    z-index: 6;
    pointer-events: none;
    border-radius: 3px;
    background: linear-gradient(
      122deg,
      transparent 42%,
      rgba(255, 255, 255, 0.1) 49%,
      rgba(255, 255, 255, 0.03) 53%,
      transparent 62%
    );
    mix-blend-mode: screen;
  }
  /* wax rosettes pressed into the bronze corners (replaces flat screws) */
  .case-screw {
    position: absolute;
    z-index: 7;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: radial-gradient(circle at 38% 32%, #d2683f, #8a2f1c 60%, #561d10);
    box-shadow:
      inset 0 1px 0 rgba(255, 200, 170, 0.5),
      inset 0 -1px 3px rgba(30, 8, 4, 0.6),
      0 1px 4px rgba(10, 4, 2, 0.55);
  }
  .case-screw::after {
    content: '❦';
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 8px;
    line-height: 1;
    color: rgba(42, 12, 6, 0.6);
  }
  .case-screw.tl { top: 8px; left: 9px; }
  .case-screw.tr { top: 8px; right: 9px; }
  .case-screw.bl { bottom: 8px; left: 9px; }
  .case-screw.br { bottom: 8px; right: 9px; }

  @media (max-width: 760px) {
    .case-inner,
    .case-inner.solo {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 430px) {
    .case-rest {
      flex-direction: column;
    }
    .case-rest .poster {
      width: 100%;
    }
  }

  .poster:focus-visible {
    outline: 2px solid rgba(231, 195, 134, 0.8);
    outline-offset: 3px;
  }

  /* narrow screens: bills go full width inside the stacked case */
  @media (max-width: 430px) {
    .case-feature {
      width: 100%;
    }
    .poster,
    .poster.featured {
      flex-basis: 100%;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .poster,
    .poster:nth-child(2n),
    .poster:nth-child(3n) {
      transform: none;
    }
    .poster:hover {
      transform: translateY(-2px);
    }
    .poster:hover :global(.poster-art img) {
      transform: none;
    }
    .poster-glow,
    .poster-enter svg,
    .poster-peel {
      transition: none;
    }
  }
</style>
