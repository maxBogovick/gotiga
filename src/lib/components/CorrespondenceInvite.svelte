<script lang="ts">
  /**
   * "Write to the author" — a light letter-card sitting right after
   * AuthorStory, the page's warmest point of contact with the maker. Not a
   * commission (that's the heavy /commission wizard, reached via
   * request-steps below) — this is the low-commitment "I just want to say
   * something" touchpoint the home page was missing.
   */
  import { t } from '$lib/i18n';
  import ContactMessageForm from '$lib/components/ContactMessageForm.svelte';

  let { authorName }: { authorName: string } = $props();
</script>

<section class="corr" aria-labelledby="corr-title">
  <div class="corr-card">
    <span class="fc fc-tl"></span>
    <span class="fc fc-tr"></span>
    <span class="fc fc-bl"></span>
    <span class="fc fc-br"></span>

    <div class="corr-intro">
      <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('homeCorrespondenceEyebrow')}</p>
      <h2 id="corr-title" class="corr-title">{$t('homeCorrespondenceTitle')}</h2>

      <blockquote class="corr-note">
        <span class="corr-quote-mark" aria-hidden="true">&ldquo;</span>
        <p>{$t('homeCorrespondenceNote')}</p>
        <cite>— {authorName}</cite>
      </blockquote>
    </div>

    <div class="corr-form">
      <ContactMessageForm source="home" />
    </div>
  </div>
</section>

<style>
  .corr {
    max-width: 900px;
    margin: clamp(24px, 4vw, 56px) auto;
    padding: 0 clamp(20px, 4.5vw, 64px);
  }

  .corr-card {
    position: relative;
    display: grid;
    grid-template-columns: 1.1fr 1fr;
    gap: clamp(28px, 4vw, 56px);
    padding: clamp(32px, 4.5vw, 52px);
    background: var(--color-canvas-raised, #f2e8d8);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-ink-primary, #34251c) 7%, transparent);
  }

  /* Letterpress corner ticks, matching AuthorStory's plate */
  .fc {
    position: absolute;
    width: 12px;
    height: 12px;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 32%, transparent);
    z-index: 2;
  }
  .fc-tl { top: 9px; left: 9px; border-right: 0; border-bottom: 0; }
  .fc-tr { top: 9px; right: 9px; border-left: 0; border-bottom: 0; }
  .fc-bl { bottom: 9px; left: 9px; border-right: 0; border-top: 0; }
  .fc-br { bottom: 9px; right: 9px; border-left: 0; border-top: 0; }

  .corr-intro {
    display: flex;
    flex-direction: column;
    justify-content: center;
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

  .corr-title {
    margin: 0 0 20px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 2.6vw, 38px);
    font-weight: 300;
    line-height: 1.05;
    color: var(--color-ink-primary, #34251c);
  }

  .corr-note {
    position: relative;
    margin: 0;
    padding-left: 4px;
  }
  .corr-quote-mark {
    position: absolute;
    top: -18px;
    left: -6px;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 48px;
    color: color-mix(in srgb, var(--color-ink-primary, #34251c) 14%, transparent);
    line-height: 1;
  }
  .corr-note p {
    margin: 0;
    max-width: 40ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(16px, 1.4vw, 19px);
    font-weight: 300;
    font-style: italic;
    line-height: 1.55;
    color: var(--color-ink-secondary, #5f4636);
  }
  .corr-note cite {
    display: block;
    margin-top: 12px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-style: normal;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-ember, #c65f3c);
  }

  .corr-form {
    display: flex;
    align-items: center;
  }

  @media (max-width: 760px) {
    .corr-card {
      grid-template-columns: 1fr;
      gap: 28px;
    }
    .corr-note p { max-width: none; }
  }
</style>
