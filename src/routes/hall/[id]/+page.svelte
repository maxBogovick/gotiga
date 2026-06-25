<script lang="ts">
  /**
   * /hall/[id] — a room of the house.
   *
   * The works pinned to one showing room, gathered on a single page. The gate is
   * already inside each tile (HomeFigurineTile): while the room sleeps its works
   * show carved sealed doors; the moment the room's window opens they reveal and
   * become enterable — live, off the house clock, no reload. This page adds only
   * the framing: the room's name, when it opens (atmospheric, no countdown), and a
   * line telling the visitor whether the doors are sealed or open right now.
   *
   * No backend of its own (load() reuses showing-rooms + the figurine list).
   */
  import { onMount } from 'svelte';
  import { t, lang, brandName } from '$lib/i18n';
  import HomeFigurineTile from '$lib/components/HomeFigurineTile.svelte';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import {
    roomToWindow,
    isShowingOpen,
    isGated,
    describeOpening,
    openingWhenLabel,
  } from '$lib/showing-window';

  let { data } = $props();

  let locale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
  let win = $derived(data.room ? roomToWindow(data.room) : null);
  let gated = $derived(!!win && isGated(win));
  let openNow = $derived(!win || isShowingOpen(win, houseClock.nowDate));

  // The atmospheric "when" for the header, while the room sleeps.
  let whenLabel = $derived.by(() => {
    if (!win || openNow) return '';
    const desc = describeOpening(win, houseClock.nowDate, 7);
    return desc ? openingWhenLabel(desc, $t, locale) : '';
  });
  let scheduleLine = $derived(
    openNow
      ? $t('hallOpenNow')
      : whenLabel
        ? $t('hallOpensWhen').split('{when}').join(whenLabel)
        : ''
  );

  onMount(() => {
    houseClock.start();
  });
</script>

<svelte:head>
  <title>{data.room ? `${data.room.name} — ${$brandName}` : $t('hallNotFound')}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main class="hall">
  <a class="hall-back" href="/">
    <svg width="14" height="7" viewBox="0 0 14 7" fill="none" aria-hidden="true">
      <path d="M14 3.5H1M1 3.5L4.5 1M1 3.5L4.5 6" stroke="currentColor" stroke-width="1" />
    </svg>
    {$t('hallBack')}
  </a>

  {#if !data.room}
    <section class="hall-empty">
      <h1>{$t('hallNotFound')}</h1>
      <p>{$t('hallNotFoundText')}</p>
    </section>
  {:else}
    <header class="hall-head">
      <p class="eyebrow"><span class="eyebrow-rule"></span>{$t('hallEyebrow')}</p>
      <h1 class="hall-name">{data.room.name}</h1>
      {#if gated}
        <p class="hall-schedule" class:is-open={openNow}>{scheduleLine}</p>
        <p class="hall-note">{openNow ? $t('hallOpenNote') : $t('hallSealedNote')}</p>
      {/if}
    </header>

    {#if data.works.length > 0}
      <div class="hall-grid">
        {#each data.works as fig, i (fig.id)}
          <HomeFigurineTile {fig} index={i} />
        {/each}
      </div>
    {:else}
      <p class="hall-empty-note">{$t('hallEmpty')}</p>
    {/if}
  {/if}
</main>

<style>
  .hall {
    max-width: 1520px;
    margin: 0 auto;
    padding:
      calc(var(--site-header-height, 68px) + clamp(20px, 3vw, 40px))
      clamp(20px, 4.5vw, 64px)
      clamp(48px, 6vw, 96px);
  }

  .hall-back {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    margin-bottom: clamp(20px, 3vw, 36px);
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    text-decoration: none;
    color: var(--color-ink-tertiary);
    transition: color 0.22s ease, gap 0.22s ease;
  }
  .hall-back svg {
    transition: transform 0.22s ease;
  }
  .hall-back:hover {
    color: var(--color-ember, #c65f3c);
  }
  .hall-back:hover svg {
    transform: translateX(-3px);
  }

  .hall-head {
    margin-bottom: clamp(26px, 3.4vw, 46px);
    padding-bottom: clamp(18px, 2.2vw, 28px);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
  }

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
    background: var(--color-ember);
    opacity: 0.65;
  }

  .hall-name {
    margin: 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(38px, 5vw, 68px);
    font-weight: 300;
    line-height: 0.96;
    color: var(--color-ink-primary);
  }

  .hall-schedule {
    margin: 16px 0 0;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }
  .hall-schedule.is-open {
    color: var(--color-ember, #c65f3c);
  }

  .hall-note {
    max-width: 46ch;
    margin: 10px 0 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(16px, 1.4vw, 19px);
    font-style: italic;
    line-height: 1.4;
    color: var(--color-ink-secondary);
  }

  .hall-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: clamp(12px, 1.4vw, 18px);
  }

  .hall-empty,
  .hall-empty-note {
    color: var(--color-ink-secondary);
  }
  .hall-empty h1 {
    margin: 0 0 10px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(34px, 4vw, 52px);
    font-weight: 300;
    color: var(--color-ink-primary);
  }
  .hall-empty p,
  .hall-empty-note {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 19px;
    font-style: italic;
  }
  .hall-empty-note {
    display: block;
    padding: 40px 0;
  }
</style>
