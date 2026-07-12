<script lang="ts">
  /**
   * VisitLedger — "Since your visit".
   *
   * A noticeable letterpress band beneath the hero that answers the returning
   * visitor's only real question: has anything changed since I was last here?
   * It diffs the collection against a private client-side snapshot (see
   * `$lib/visit-ledger`) and surfaces the three honest change signals the public
   * API exposes — arrivals, works that have found a home, and halls beginning to
   * wake. When nothing concrete has changed it does NOT fall silent: it reframes
   * the passing of time ("the collection rests, but the workshop's dust never
   * settles"), so the museum always feels alive rather than frozen.
   *
   * Shows nothing on a first visit (no baseline to compare against). Today's
   * vitrine pick used to ride along here as a compact mark, but now gets its
   * own pinned specimen card in the hero (visible to every visitor, not just
   * returning ones) — see `pinnedSpecimenFig` in `routes/+page.svelte`.
   */
  import { t } from '$lib/i18n';
  import type { FigurineListItem } from '$lib/types/api';
  import { diffVisit, commitVisit, type RoomRef, type VisitChanges } from '$lib/visit-ledger';

  let {
    figurines = [],
    rooms = [],
    inProgressCount = 0,
  }: {
    figurines?: FigurineListItem[];
    rooms?: RoomRef[];
    inProgressCount?: number;
  } = $props();

  let changes = $state<VisitChanges | null>(null);
  let settled = false;

  // Compute the diff once, the first time a real payload arrives, then record
  // this visit as the new baseline. The read must precede the write.
  $effect(() => {
    if (settled || figurines.length === 0) return;
    settled = true;
    changes = diffVisit(figurines, rooms);
    commitVisit(figurines, rooms);
  });

  let show = $derived(changes != null && !changes.firstVisit);
  let showLedger = $derived(show);
  let homed = $derived(changes?.homed.slice(0, 2) ?? []);
  let newRooms = $derived(changes?.newRooms.slice(0, 2) ?? []);
  let arrivals = $derived(changes?.arrivals.length ?? 0);
  let arrivalsHref = $derived(
    changes?.arrivals.length === 1 ? `/figurines/${changes.arrivals[0].id}` : '/figurines'
  );
  let updated = $derived(changes?.updated.length ?? 0);
  let updatedHref = $derived(
    changes?.updated.length === 1 ? `/figurines/${changes.updated[0].id}` : '/figurines'
  );
</script>

{#if showLedger}
  <aside class="ledger" aria-label={$t('homeLedgerEyebrow')}>
    <div class="ledger-rule" aria-hidden="true"></div>

    <p class="ledger-eyebrow">
      <span class="ledger-fleuron" aria-hidden="true">❧</span>
      {$t('homeLedgerEyebrow')}
      {#if changes?.daysSince && changes.daysSince > 0}
        <span class="ledger-days">{changes.daysSince} {$t('homeLedgerDays')}</span>
      {/if}
    </p>

    <div class="ledger-marks">

      {#if show && changes?.hasAny}
        {#if arrivals > 0}
          <a class="mark mark-link" href={arrivalsHref}>
            <span class="mark-num">{arrivals}</span>
            <span class="mark-label">{$t('homeLedgerArrived')}</span>
            <span class="mark-cta">{$t('homeLedgerArrivedCta')} →</span>
          </a>
        {/if}

        {#each homed as f (f.id)}
          <a class="mark mark-link" href={`/figurines/${f.id}`}>
            <span class="mark-seal" aria-hidden="true">✦</span>
            <span class="mark-named">«{f.name}»</span>
            <span class="mark-label">{$t('homeLedgerHomed')}</span>
          </a>
        {/each}

        {#if updated > 0}
          <a class="mark mark-link" href={updatedHref}>
            <span class="mark-num">{updated}</span>
            <span class="mark-label">{$t('homeLedgerUpdated')}</span>
            <span class="mark-cta">{$t('homeLedgerUpdatedCta')} →</span>
          </a>
        {/if}

        {#each newRooms as r (r.id)}
          <a class="mark mark-link" href={`/hall/${r.id}`}>
            <span class="mark-seal" aria-hidden="true">❦</span>
            <span class="mark-named">«{r.name}»</span>
            <span class="mark-label">{$t('homeLedgerRoomWaking')}</span>
          </a>
        {/each}
      {/if}

      {#if show && !changes?.hasAny && inProgressCount > 0}
        <a class="mark mark-link mark-inprogress" href="/upcoming">
          <span class="mark-num">{inProgressCount}</span>
          <span class="mark-label">{$t('homeLedgerInProgress')}</span>
          <span class="mark-cta">{$t('homeLedgerInProgressCta')} →</span>
        </a>
      {/if}
    </div>

    {#if show && !changes?.hasAny}
      <div class="ledger-quiet">
        <p class="quiet-line">{$t('homeLedgerQuiet')}</p>
        <a class="quiet-cta" href="/workshop">
          {$t('homeLedgerQuietCta')}
          <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1" />
          </svg>
        </a>
      </div>
    {/if}
  </aside>
{/if}

<style>
  .ledger {
    position: relative;
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(16px, 2vw, 24px) clamp(20px, 4.5vw, 64px) clamp(14px, 1.8vw, 20px);
  }

  /* Double engraved hairline above — a ledger entry ruled off from the hero. */
  .ledger-rule {
    height: 0;
    margin-bottom: clamp(14px, 1.6vw, 20px);
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
    box-shadow: 0 3px 0 -2px color-mix(in srgb, var(--color-ink-primary) 9%, transparent);
  }

  .ledger-eyebrow {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 14px;
    margin: 0 0 clamp(10px, 1.3vw, 16px);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  .ledger-fleuron {
    color: var(--color-ember);
    font-size: 14px;
    line-height: 1;
  }

  .ledger-days {
    font-weight: 400;
    letter-spacing: 0.04em;
    text-transform: none;
    font-style: italic;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 15px;
    color: color-mix(in srgb, var(--color-ink-tertiary) 88%, transparent);
  }

  .ledger-marks {
    display: flex;
    flex-wrap: wrap;
    gap: clamp(10px, 1.4vw, 18px);
  }

  .mark {
    display: inline-flex;
    align-items: baseline;
    gap: 9px;
    padding: 9px 15px;
    text-decoration: none;
    color: var(--color-ink-primary);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    border-left: 2px solid color-mix(in srgb, var(--color-ember) 64%, transparent);
    background:
      linear-gradient(180deg,
        color-mix(in srgb, var(--color-canvas-raised) 86%, transparent),
        color-mix(in srgb, var(--color-canvas-raised) 52%, transparent));
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.6) inset;
    transition: border-color 0.22s ease, transform 0.16s ease, background 0.22s ease;
  }

  .mark-link:hover {
    transform: translateY(-1px);
    border-left-color: var(--color-ember);
    background:
      linear-gradient(180deg,
        color-mix(in srgb, var(--color-canvas-raised) 96%, transparent),
        color-mix(in srgb, var(--color-canvas-raised) 64%, transparent));
  }

  .mark-link:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--color-ember) 56%, transparent);
    outline-offset: 3px;
  }

  .mark-num {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(26px, 2.2vw, 32px);
    font-weight: 300;
    line-height: 0.8;
    color: var(--color-ember-deep);
  }

  .mark-seal {
    font-size: 14px;
    line-height: 1;
    color: var(--color-ember);
    align-self: center;
  }

  .mark-named {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.5vw, 21px);
    font-style: italic;
    font-weight: 300;
    line-height: 1;
    color: var(--color-ink-primary);
  }

  .mark-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  .mark-cta {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ember-deep);
  }

  /* Quiet fallback — nothing changed, but time still passed. */
  .ledger-quiet {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 8px 20px;
  }

  .mark-inprogress {
    border-left-color: color-mix(in srgb, var(--color-ochre, #c9a875) 64%, transparent);
  }

  .quiet-line {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.6vw, 22px);
    font-style: italic;
    font-weight: 300;
    line-height: 1.32;
    color: var(--color-ink-secondary);
  }

  .quiet-cta {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    text-decoration: none;
    color: var(--color-ember-deep);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember) 40%, transparent);
    padding-bottom: 2px;
    transition: color 0.2s ease, border-color 0.2s ease;
  }

  .quiet-cta:hover {
    color: var(--color-ember);
    border-color: var(--color-ember);
  }

  @media (prefers-reduced-motion: reduce) {
    .mark-link:hover { transform: none; }
  }
</style>
