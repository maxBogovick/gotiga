<script lang="ts">
  /**
   * "Замечено гостями" — a hybrid editorial+algorithmic shelf (the artisan's
   * own pins first, remaining slots auto-filled from the private mark
   * ranking). Deliberately NOT a public leaderboard: no counts, no visible
   * ranking order beyond "these were noticed" — the server already resolved
   * the list server-side (see /figurines/noticed), this component only renders
   * what it's given. Self-hides when the shelf is empty (no pins yet and not
   * enough mark signal), same convention as FirstLook/MarkedByYou.
   */
  import { t } from '$lib/i18n';
  import type { FigurineListItem } from '$lib/types/api';
  import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';

  let { figurines = [] }: { figurines?: FigurineListItem[] } = $props();
</script>

{#if figurines.length > 0}
  <section class="noticed-by-guests" aria-labelledby="noticed-by-guests-title">
    <div class="nbg-head">
      <div>
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('noticedByGuestsEyebrow')}</p>
        <h2 id="noticed-by-guests-title" class="nbg-title">{$t('noticedByGuestsTitle')}</h2>
      </div>
      <p class="nbg-lead">{$t('noticedByGuestsLead')}</p>
    </div>

    <div class="nbg-grid" class:nbg-grid-short={figurines.length <= 3}>
      {#each figurines as fig, i (fig.id)}
        <HomeFigurineTile {fig} index={i} />
      {/each}
    </div>
  </section>
{/if}

<style>
  .noticed-by-guests {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(20px, 3vw, 40px) clamp(20px, 4.5vw, 64px) clamp(8px, 1.5vw, 16px);
  }

  .nbg-head {
    display: grid;
    grid-template-columns: minmax(220px, 0.42fr) minmax(0, 0.58fr);
    gap: clamp(18px, 2.4vw, 36px);
    align-items: end;
    margin-bottom: 16px;
    padding-bottom: 14px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border-default, #d8c6b1) 60%, transparent);
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
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
    background: var(--color-ink-tertiary);
    opacity: 0.6;
  }

  .nbg-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 2.7vw, 40px);
    font-weight: 300;
    line-height: 0.98;
    color: var(--color-ink-primary, #34251c);
  }

  .nbg-lead {
    margin: 0;
    max-width: 52ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(15px, 1.3vw, 19px);
    font-style: italic;
    font-weight: 300;
    line-height: 1.45;
    color: var(--color-ink-secondary, #5f4636);
  }

  .nbg-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: clamp(10px, 1.2vw, 16px);
  }
  .nbg-grid > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: 0 380px;
  }
  .nbg-grid-short {
    grid-template-columns: repeat(auto-fit, minmax(250px, 320px));
  }

  @media (max-width: 760px) {
    .nbg-head { grid-template-columns: 1fr; align-items: start; }
  }
</style>
