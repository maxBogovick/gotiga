<script lang="ts">
  /**
   * "Отмеченное вами" — the visitor's own private collecting ritual, not a
   * public leaderboard. Every mark a visitor leaves on a figurine page
   * (see visitor-marks.svelte.ts) is invisible to everyone else; this band
   * is the one place it comes back to them — a personal cabinet-of-curios
   * page, not a social signal. This is deliberately the return-visit driver
   * instead of a public "top figurines" ranking (see project decision:
   * public counts risk vanity-metric / negative-social-proof effects on
   * unique, often already-sold pieces).
   *
   * Self-hides when the visitor has marked nothing yet — same convention as
   * FirstLook, which this is structurally modeled on. Unlike FirstLook, this
   * is NOT gated behind visitorBook.signed: marking works fully anonymously.
   */
  import { t } from '$lib/i18n';
  import type { FigurineListItem } from '$lib/types/api';
  import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';

  let { figurines = [] }: { figurines?: FigurineListItem[] } = $props();
</script>

{#if figurines.length > 0}
  <section id="marked-by-you" class="marked-by-you" aria-labelledby="marked-by-you-title">
    <div class="mby-head">
      <div>
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('markedByYouEyebrow')}</p>
        <h2 id="marked-by-you-title" class="mby-title">{$t('markedByYouTitle')}</h2>
      </div>
      <p class="mby-lead">{$t('markedByYouLead')}</p>
    </div>

    <div class="mby-grid" class:mby-grid-short={figurines.length <= 3}>
      {#each figurines as fig, i (fig.id)}
        <HomeFigurineTile {fig} index={i} />
      {/each}
    </div>

    <p class="mby-note">❧ {$t('markedByYouNote')}</p>
  </section>
{/if}

<style>
  .marked-by-you {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(20px, 3vw, 40px) clamp(20px, 4.5vw, 64px) clamp(8px, 1.5vw, 16px);
    scroll-margin-top: calc(var(--site-header-height, 68px) + 12px);
  }

  .mby-head {
    display: grid;
    grid-template-columns: minmax(220px, 0.42fr) minmax(0, 0.58fr);
    gap: clamp(18px, 2.4vw, 36px);
    align-items: end;
    margin-bottom: 16px;
    padding-bottom: 14px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember) 34%, transparent);
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
    color: var(--color-ember-deep, #6f3b24);
  }
  .eyebrow-rule {
    display: inline-block;
    width: 26px;
    height: 1px;
    background: var(--color-ember, #c65f3c);
    opacity: 0.8;
  }

  .mby-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 2.7vw, 40px);
    font-weight: 300;
    line-height: 0.98;
    color: var(--color-ink-primary, #34251c);
  }

  .mby-lead {
    margin: 0;
    max-width: 52ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(15px, 1.3vw, 19px);
    font-style: italic;
    font-weight: 300;
    line-height: 1.45;
    color: var(--color-ink-secondary, #5f4636);
  }

  .mby-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: clamp(10px, 1.2vw, 16px);
  }
  .mby-grid > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: 0 380px;
  }
  .mby-grid-short {
    grid-template-columns: repeat(auto-fit, minmax(250px, 320px));
  }

  .mby-note {
    margin: 14px 0 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 14px;
    line-height: 1.4;
    color: var(--color-ink-tertiary);
  }

  @media (max-width: 760px) {
    .mby-head { grid-template-columns: 1fr; align-items: start; }
  }
</style>
