<script lang="ts">
  /**
   * The cabinet blotter frame: corners, eyebrow, title, lead, writing line.
   * Same object in the gallery and under the header when the loupe recalls it.
   */
  import { t } from '$lib/i18n';
  import KeeperBlotter from '$lib/components/KeeperBlotter.svelte';

  type Props = {
    titleId: string;
    source?: string;
    autofocus?: boolean;
    float?: boolean;
  };

  let { titleId, source = 'home_keeper', autofocus = false, float = false }: Props = $props();
</script>

<section
  class="keeper-note"
  class:is-float={float}
  aria-labelledby={titleId}
>
  <span class="fc fc-tl"></span>
  <span class="fc fc-tr"></span>
  <span class="fc fc-bl"></span>
  <span class="fc fc-br"></span>

  <p class="kn-eyebrow"><span class="kn-rule"></span>{$t('homeKeeperEyebrow')}</p>
  <h3 id={titleId} class="kn-title">{$t('homeKeeperTitle')}</h3>
  <p class="kn-lead">{$t('homeKeeperLead')}</p>

  <KeeperBlotter {source} {autofocus} />
</section>

<style>
  .keeper-note {
    position: relative;
    width: 100%;
    max-width: var(--reel-card-width, 64rem);
    box-sizing: border-box;
    margin: 0 auto;
    padding: clamp(22px, 3vw, 36px) clamp(20px, 3.2vw, 40px) clamp(20px, 2.6vw, 32px);
    background: color-mix(in srgb, var(--color-canvas-raised, #f2e8d8) 88%, #fff 12%);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-ink-primary, #34251c) 6%, transparent);
  }

  .keeper-note.is-float {
    max-height: min(70vh, calc(100vh - var(--site-header-h, 54px) - 24px));
    overflow-x: hidden;
    overflow-y: auto;
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--color-ink-primary, #34251c) 6%, transparent),
      0 18px 36px color-mix(in srgb, #34251c 10%, transparent);
  }

  .fc {
    position: absolute;
    width: 11px;
    height: 11px;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 30%, transparent);
    z-index: 2;
    pointer-events: none;
  }
  .fc-tl { top: 8px; left: 8px; border-right: 0; border-bottom: 0; }
  .fc-tr { top: 8px; right: 8px; border-left: 0; border-bottom: 0; }
  .fc-bl { bottom: 8px; left: 8px; border-right: 0; border-top: 0; }
  .fc-br { bottom: 8px; right: 8px; border-left: 0; border-top: 0; }

  .kn-eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 8px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #7c6554;
  }
  .kn-rule {
    width: 28px;
    height: 1px;
    background: #c65f3c;
  }

  .kn-title {
    margin: 0 0 6px;
    font-family: 'Fraunces', 'Georgia', serif;
    font-size: clamp(1.35rem, 2.4vw, 1.85rem);
    font-weight: 400;
    color: #6f3b24;
  }

  .kn-lead {
    margin: 0 0 18px;
    font-family: 'Georgia', serif;
    font-style: italic;
    font-size: 0.95rem;
    line-height: 1.45;
    color: #7c6554;
    max-width: 36rem;
  }
</style>
