<script lang="ts">
  /**
   * FirstLook — the book-holders' privilege, made visible.
   *
   * "Be the first to see" is the single most cited benefit of a museum membership,
   * and the strongest driver of return visits. This band delivers that on-site: a
   * curated shelf the house lays out *for those in the book* — the newest works and
   * what is still on the bench — set apart and named as a courtesy, not a sale.
   *
   * Rendered by the home page only when `visitorBook.signed`, so an anonymous
   * visitor never sees it. v1 is recognition-framed curation of works already in
   * the archive; a true timed early-release (a per-work release date that hides a
   * piece from the public archive until its hour) is the planned v2 — at which
   * point this same band becomes a genuine temporal preview.
   */
  import { t, lang } from '$lib/i18n';
  import type { FigurineListItem } from '$lib/types/api';
  import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';

  let {
    works = [],
    greetName = '',
    source,
  }: {
    works?: FigurineListItem[];
    greetName?: string;
    // Tags each tile's link for admin analytics (e.g. "home_first_look").
    // Optional — other callers of this component simply omit it.
    source?: string;
  } = $props();

  let locale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
  function opensLabel(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleDateString(locale, { day: 'numeric', month: 'long' });
  }
</script>

{#if works.length > 0}
  <section id="first-look" class="first-look" aria-labelledby="first-look-title">
    <div class="fl-head">
      <div>
        <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('firstLookEyebrow')}</p>
        <h2 id="first-look-title" class="fl-title">{$t('firstLookTitle')}</h2>
      </div>
      <p class="fl-lead">
        {#if greetName}<span class="fl-name">{greetName},</span> {/if}{$t('firstLookLead')}
      </p>
    </div>

    <div class="fl-grid" class:fl-grid-short={works.length <= 3}>
      {#each works as fig, i (fig.id)}
        <div class="fl-item">
          <HomeFigurineTile {fig} index={i} {source} />
          {#if fig.firstLookUntil}
            <p class="fl-opens">
              <span class="fl-opens-dot" aria-hidden="true"></span>
              {$t('firstLookOpens')} {opensLabel(fig.firstLookUntil)}
            </p>
          {/if}
        </div>
      {/each}
    </div>

    <p class="fl-note">❧ {$t('firstLookNote')}</p>
  </section>
{/if}

<style>
  .first-look {
    max-width: 1520px;
    margin: 0 auto;
    padding: clamp(20px, 3vw, 40px) clamp(20px, 4.5vw, 64px) clamp(8px, 1.5vw, 16px);
  }

  .fl-head {
    display: grid;
    grid-template-columns: minmax(220px, 0.42fr) minmax(0, 0.58fr);
    gap: clamp(18px, 2.4vw, 36px);
    align-items: end;
    margin-bottom: 16px;
    padding-bottom: 14px;
    /* a warm seam, distinct from the plain section rules elsewhere, marks this as
       a set-apart courtesy shelf */
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

  .fl-title {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(28px, 2.7vw, 40px);
    font-weight: 300;
    line-height: 0.98;
    color: var(--color-ink-primary, #34251c);
  }

  .fl-lead {
    margin: 0;
    max-width: 52ch;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(15px, 1.3vw, 19px);
    font-style: italic;
    font-weight: 300;
    line-height: 1.45;
    color: var(--color-ink-secondary, #5f4636);
  }
  .fl-name {
    font-style: normal;
    color: var(--color-ember-deep, #6f3b24);
  }

  .fl-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: clamp(10px, 1.2vw, 16px);
  }
  .fl-grid > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: 0 380px;
  }
  .fl-grid-short {
    grid-template-columns: repeat(auto-fit, minmax(250px, 320px));
  }

  .fl-item {
    display: grid;
    gap: 8px;
    align-content: start;
  }
  .fl-opens {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding-left: 2px;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-ember-deep, #6f3b24);
  }
  .fl-opens-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-ember, #c65f3c);
    flex-shrink: 0;
  }

  .fl-note {
    margin: 14px 0 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-style: italic;
    font-size: 14px;
    line-height: 1.4;
    color: var(--color-ink-tertiary);
  }

  @media (max-width: 760px) {
    .fl-head { grid-template-columns: 1fr; align-items: start; }
  }
</style>
