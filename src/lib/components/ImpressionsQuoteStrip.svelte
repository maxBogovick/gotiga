<script lang="ts">
  /**
   * ImpressionsQuoteStrip — public face of the Book of Impressions. Unlike
   * FirstLook/MarkedByYou/NoticedByGuests, this section does NOT self-hide
   * when empty: it is the only entry point to /impressions on the home page,
   * so the invitation must always be visible, even before the first quote is
   * curated. With quotes: a row of curator-picked visitor reactions. Without:
   * a plain invitation card, same eyebrow/title, leading to the same CTA.
   */
  import { onMount } from 'svelte';
  import { t, type TranslationKey } from '$lib/i18n';
  import { api } from '$lib/api';
  import type { ImpressionDto } from '$lib/types/api';

  let impressions = $state<ImpressionDto[]>([]);

  onMount(async () => {
    impressions = await api.getFeaturedImpressions();
  });

  const MOOD_LABEL_KEYS: Record<string, TranslationKey> = {
    quiet: 'impressionsMood_quiet',
    haunting: 'impressionsMood_haunting',
    nostalgic: 'impressionsMood_nostalgic',
    meticulous: 'impressionsMood_meticulous',
    uneasy: 'impressionsMood_uneasy',
    moved: 'impressionsMood_moved',
  };
  function moodLabel(mood: string): string {
    const key = MOOD_LABEL_KEYS[mood];
    return key ? $t(key) : mood;
  }
</script>

<section class="quote-strip" aria-labelledby="impressions-strip-title">
  <div class="strip-head">
    <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('impressionsStripEyebrow')}</p>
    <h2 id="impressions-strip-title" class="strip-title">{$t('impressionsStripTitle')}</h2>
  </div>

  {#if impressions.length > 0}
    <ul class="quote-list">
      {#each impressions as im (im.id)}
        <li class="quote-card">
          <span class="quote-mark" aria-hidden="true">❝</span>
          <p class="quote-text">{im.message}</p>
          <p class="quote-attrib">
            {#if im.mood}<span class="quote-mood">{moodLabel(im.mood)}</span> · {/if}
            {im.authorName || $t('impressionsAnonymous')}
          </p>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="quote-empty">{$t('impressionsStripEmpty')}</p>
  {/if}

  <a class="leave-cta" href="/impressions">
    {$t('impressionsStripCta')}
    <svg width="18" height="9" viewBox="0 0 18 9" fill="none" aria-hidden="true">
      <path d="M0 4.5H17M17 4.5L12.5 1M17 4.5L12.5 8" stroke="currentColor" stroke-width="1"/>
    </svg>
  </a>
</section>

<style>
  .quote-strip {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(20px, 3vw, 40px) clamp(20px, 4.5vw, 64px) clamp(48px, 6vw, 88px);
  }

  .strip-head { margin-bottom: clamp(20px, 3vw, 32px); }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
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
    background: var(--color-ember, #c65f3c);
    opacity: 0.65;
  }

  .strip-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 2.8vw, 42px);
    font-weight: 300;
    line-height: 1;
    color: var(--color-ink-primary, #34251c);
  }

  .quote-list {
    list-style: none;
    margin: 0 0 28px;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: clamp(20px, 2.5vw, 32px);
  }

  .quote-empty {
    margin: 0 0 28px;
    max-width: 46ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: clamp(16px, 1.4vw, 19px);
    line-height: 1.5;
    color: var(--color-ink-secondary, #5f4636);
  }

  .quote-card {
    position: relative;
    padding: 20px 22px 18px;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    border-radius: 3px;
    background: linear-gradient(180deg, var(--color-canvas-raised, #fffaf2), var(--color-canvas-base, #f8f1e7));
  }

  .quote-mark {
    display: block;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 32px;
    line-height: 1;
    color: var(--color-ember, #c65f3c);
    opacity: 0.6;
    margin-bottom: 4px;
  }

  .quote-text {
    margin: 0 0 14px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: clamp(16px, 1.4vw, 19px);
    line-height: 1.5;
    color: var(--color-ink-primary, #34251c);
  }

  .quote-attrib {
    margin: 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.03em;
    color: var(--color-ink-tertiary);
  }

  .quote-mood {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-ember-deep, #6f3b24);
  }

  .leave-cta {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    height: 42px;
    padding: 0 22px;
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 28%, transparent);
    color: var(--color-ink-primary, #34251c);
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    text-decoration: none;
    transition: border-color 0.2s ease, color 0.2s ease, transform 0.12s ease;
  }
  .leave-cta svg { transition: transform 0.22s ease; }
  .leave-cta:hover {
    border-color: var(--color-ember, #c65f3c);
    color: var(--color-ember-deep, #6f3b24);
  }
  .leave-cta:hover svg { transform: translateX(4px); }
  .leave-cta:active { transform: translateY(1px); }
</style>
