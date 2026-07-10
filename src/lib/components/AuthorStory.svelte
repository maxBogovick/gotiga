<script lang="ts">
  /**
   * Author-led home (Concept B), block 2 — "The maker behind the figures".
   * Elevates the person: a framed portrait beside a short first-person note,
   * with the paths onward to the full author page and the workshop. Content is
   * reused from the existing admin-editable AuthorProfile (name / bio / photo);
   * i18n keys provide the fallback copy shown until the profile is filled in.
   * This block is core to the page's job (reputation → commission), so it never
   * self-hides — a placeholder plate holds the layout while there is no photo.
   */
  import { t } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    name,
    bio = null,
    photoUrl = null,
  }: { name: string; bio?: string | null; photoUrl?: string | null } = $props();

  let bodyText = $derived(bio?.trim() || $t('homeStoryBody'));
  let initial = $derived((name?.trim()?.[0] ?? '').toUpperCase());
</script>

<section id="in-the-workshop" class="author-story" aria-labelledby="author-story-title">
  <div class="as-portrait">
    <div class="as-plate">
      {#if photoUrl}
        <AppImage src={photoUrl} alt={$t('homeStoryPortraitAlt')} class="as-img" />
      {:else}
        <div class="as-placeholder" aria-hidden="true">
          <span class="as-placeholder-mark">{initial || '·'}</span>
          <span class="as-placeholder-note">{$t('homeStoryPortraitAlt')}</span>
        </div>
      {/if}
      <span class="fc fc-tl"></span>
      <span class="fc fc-tr"></span>
      <span class="fc fc-bl"></span>
      <span class="fc fc-br"></span>
    </div>
  </div>

  <div class="as-copy">
    <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('homeStoryEyebrow')}</p>
    <h2 id="author-story-title" class="as-title">{$t('homeStoryTitle')}</h2>
    <p class="as-name">{name}</p>
    <p class="as-body">{bodyText}</p>
    <div class="as-actions">
      <a href="/author" class="as-link as-link-primary">
        {$t('homeStoryMore')}
        <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
          <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1" />
        </svg>
      </a>
      <a href="/workshop" class="as-link as-link-ghost">{$t('homeWorkshopTeaserLabel')}</a>
    </div>
  </div>
</section>

<style>
  .author-story {
    position: relative;
    max-width: 1520px;
    margin: clamp(24px, 4vw, 56px) auto;
    min-height: clamp(440px, 46vw, 680px);
    padding: 0 clamp(20px, 4.5vw, 64px);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    scroll-margin-top: var(--site-header-height, 68px);
  }

  /* ── Portrait plate ─────────────────────────── */
  /* The tall photo bleeds the full height of the band and slides under the copy
     on its right, where it dissolves into parchment — so the part that doesn't
     fit the frame still shows through behind the text block. Tune the reveal
     with --as-reveal on the section (0 = flush, higher = more overlap). */
  .as-portrait {
    position: absolute;
    top: 0;
    bottom: 0;
    left: clamp(20px, 4.5vw, 64px);
    width: min(64%, 780px);
    z-index: 1;
  }
  .as-plate {
    position: absolute;
    inset: 0;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-ink-primary, #34251c) 7%, transparent);
    background: var(--color-canvas-raised, #f2e8d8);
    overflow: hidden;
  }
  .as-plate :global(.as-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    /* keep the face near the top; the long photo continues downward */
    object-position: center 15%;
    display: block;
    filter: saturate(0.94) contrast(1.02);
  }
  /* right edge of the photo fades to parchment so the copy reads over it */
  .as-portrait::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(
      60deg,
      transparent 0%,
      transparent 60%,
      color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 80%, transparent) 95%,
      var(--color-canvas-base, #f8f1e7) 100%
    );
  }
  .as-placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    background:
      radial-gradient(120% 90% at 50% 20%, color-mix(in srgb, var(--color-ink-primary, #34251c) 5%, transparent), transparent 70%),
      var(--color-canvas-raised, #f2e8d8);
  }
  .as-placeholder-mark {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(56px, 8vw, 104px);
    font-weight: 300;
    line-height: 1;
    color: color-mix(in srgb, var(--color-ink-primary, #34251c) 42%, transparent);
  }
  .as-placeholder-note {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #6f5a48);
  }

  /* Letterpress corner ticks, echoing the hero frame */
  .fc {
    position: absolute;
    width: 12px;
    height: 12px;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 32%, transparent);
    z-index: 2;
  }
  .fc-tl { top: 7px; left: 7px; border-right: 0; border-bottom: 0; }
  .fc-tr { top: 7px; right: 7px; border-left: 0; border-bottom: 0; }
  .fc-bl { bottom: 7px; left: 7px; border-right: 0; border-top: 0; }
  .fc-br { bottom: 7px; right: 7px; border-left: 0; border-top: 0; }

  /* ── Copy ───────────────────────────────────── */
  .as-copy {
    position: relative;
    z-index: 2;
    width: min(50%, 600px);
    padding: clamp(28px, 3vw, 48px) 0 clamp(28px, 3vw, 48px) clamp(26px, 3vw, 48px);
    /* Parchment veil: translucent at the very left so the photo seeps through at
       the seam, fully opaque under the text lines so the copy stays crisp. */
    background: linear-gradient(
      90deg,
      transparent 0%,
      color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 70%, transparent) 9%,
      var(--color-canvas-base, #f8f1e7) 20%
    );
  }

  .eyebrow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary, #6f5a48);
  }
  .eyebrow-rule {
    display: inline-block;
    width: 26px;
    height: 1px;
    background: var(--color-ink-tertiary, #6f5a48);
    opacity: 0.6;
  }

  .as-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(30px, 3.2vw, 48px);
    font-weight: 300;
    line-height: 1;
    color: var(--color-ink-primary, #34251c);
  }
  .as-name {
    margin: 12px 0 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-ember, #c65f3c);
  }
  .as-body {
    margin: 18px 0 0;
    max-width: 56ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(17px, 1.5vw, 22px);
    font-weight: 300;
    line-height: 1.5;
    color: var(--color-ink-secondary, #5f4636);
  }

  .as-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 22px;
    margin-top: clamp(20px, 2.4vw, 30px);
  }
  .as-link {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-decoration: none;
    transition: color 0.25s ease, gap 0.25s ease;
  }
  .as-link svg { transition: transform 0.25s ease; }
  .as-link-primary { color: var(--color-ink-primary, #34251c); }
  .as-link-primary:hover { gap: 13px; color: var(--color-ember, #c65f3c); }
  .as-link-ghost {
    color: var(--color-ink-tertiary, #6f5a48);
    padding-bottom: 2px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 22%, transparent);
  }
  .as-link-ghost:hover { color: var(--color-ink-primary, #34251c); }

  @media (max-width: 860px) {
    /* No room to bleed under the copy on narrow screens: stack the tall photo
       above the text at its natural crop instead. */
    .author-story {
      display: block;
      min-height: 0;
      padding: clamp(20px, 5vw, 32px) clamp(20px, 4.5vw, 64px);
    }
    .as-portrait {
      position: relative;
      inset: auto;
      left: 0;
      width: 100%;
      max-width: 420px;
      margin: 0 auto clamp(20px, 5vw, 32px);
    }
    .as-plate { position: relative; aspect-ratio: 4 / 5; }
    .as-portrait::after { display: none; }
    .as-copy {
      width: 100%;
      padding: 0;
      background: none;
    }
  }
</style>
