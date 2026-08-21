<script lang="ts">
  /**
   * End-of-leaf neighbours. The header plates greet a visitor; the edge
   * chevrons hint while they read; this is the invitation after the text —
   * the same quiet pair the gazette uses, not a shop's "related products".
   */
  import type { FigurineListItem } from '$lib/types/api';
  import { figurineHref } from '$lib/figurineHref';
  import AppImage from '$lib/components/AppImage.svelte';
  import { t } from '$lib/i18n';
  import { pageTurn } from '$lib/stores/page-turn.svelte';
  import { turnSound } from '$lib/stores/page-turn-sound.svelte';
  import { playTurnSound } from '$lib/audio/page-turn-sounds';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { isGated, isShowingOpen, resolveWindow } from '$lib/showing-window';

  let {
    prev = null,
    next = null,
  }: {
    prev?: FigurineListItem | null;
    next?: FigurineListItem | null;
  } = $props();

  function sealed(work: FigurineListItem | null) {
    if (!work) return false;
    const win = resolveWindow(work, showingRooms.list);
    return isGated(win) && !isShowingOpen(win, houseClock.nowDate);
  }

  function arm(e: MouseEvent, direction: 'forward' | 'backward') {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if (typeof window !== 'undefined' && !window.matchMedia('(pointer: fine)').matches) return;
    pageTurn.arm(direction);
    const sound = turnSound.value;
    if (sound !== 'off') playTurnSound(sound, direction);
  }
</script>

{#if prev || next}
  <nav class="leaf-turn" aria-label={$t('figurineLeafNav')}>
    {#if prev}
      <a
        class="leaf-turn-card"
        href={figurineHref(prev)}
        data-sveltekit-preload-data="hover"
        onclick={(e) => arm(e, 'backward')}
      >
        <span class="leaf-turn-mat">
          {#if sealed(prev)}
            <span class="leaf-turn-seal" aria-hidden="true"></span>
          {:else}
            <AppImage
              src={prev.faceImageUrl}
              thumbUrl={prev.thumbUrl}
              alt=""
              class="leaf-turn-img"
              sizes="72px"
              width={72}
              height={72}
              loading="lazy"
            />
          {/if}
        </span>
        <span class="leaf-turn-copy">
          <span class="leaf-turn-kicker">{$t('figurineLeafPrev')}</span>
          <span class="leaf-turn-name">{prev.name}</span>
        </span>
      </a>
    {:else}
      <span class="leaf-turn-card leaf-turn-card--empty" aria-hidden="true"></span>
    {/if}

    {#if next}
      <a
        class="leaf-turn-card leaf-turn-card--next"
        href={figurineHref(next)}
        data-sveltekit-preload-data="hover"
        onclick={(e) => arm(e, 'forward')}
      >
        <span class="leaf-turn-copy">
          <span class="leaf-turn-kicker">{$t('figurineLeafNext')}</span>
          <span class="leaf-turn-name">{next.name}</span>
        </span>
        <span class="leaf-turn-mat">
          {#if sealed(next)}
            <span class="leaf-turn-seal" aria-hidden="true"></span>
          {:else}
            <AppImage
              src={next.faceImageUrl}
              thumbUrl={next.thumbUrl}
              alt=""
              class="leaf-turn-img"
              sizes="72px"
              width={72}
              height={72}
              loading="lazy"
            />
          {/if}
        </span>
      </a>
    {/if}
  </nav>
{/if}

<style>
  .leaf-turn {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px;
    margin: 48px 0 8px;
    padding-top: 22px;
    border-top: 1px solid color-mix(in srgb, var(--color-border-default, #d8c6b1) 70%, transparent);
  }

  .leaf-turn-card {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    text-decoration: none;
    color: inherit;
  }

  .leaf-turn-card--next {
    justify-content: flex-end;
    text-align: right;
  }

  .leaf-turn-card--empty {
    visibility: hidden;
  }

  .leaf-turn-mat {
    flex: 0 0 auto;
    width: 56px;
    height: 56px;
    overflow: hidden;
    border-radius: 2px;
    background: color-mix(in srgb, var(--color-canvas-sunken, #ede3cf) 70%, transparent);
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--copper, #c65f3c) 45%, transparent),
      0 0 0 3px color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 92%, white),
      0 0 0 4px color-mix(in srgb, var(--copper, #c65f3c) 22%, transparent);
    transform: rotate(-1deg);
    transition: transform 0.28s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .leaf-turn-card--next .leaf-turn-mat {
    transform: rotate(1deg);
  }

  .leaf-turn-card:hover .leaf-turn-mat,
  .leaf-turn-card:focus-visible .leaf-turn-mat {
    transform: rotate(0deg) scale(1.04);
  }

  .leaf-turn-card:focus-visible {
    outline: none;
  }

  .leaf-turn-img {
    width: 100%;
    height: 100%;
  }

  .leaf-turn-img :global(.app-image-main) {
    object-fit: cover;
  }

  .leaf-turn-seal {
    display: block;
    width: 100%;
    height: 100%;
    background:
      radial-gradient(ellipse at 50% 42%, color-mix(in srgb, var(--ink, #34251c) 18%, transparent), transparent 62%),
      color-mix(in srgb, var(--ink, #34251c) 72%, #6f3b24);
  }

  .leaf-turn-copy {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .leaf-turn-kicker {
    font-family: var(--font-body, system-ui, sans-serif);
    font-size: 0.56rem;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-ink-muted, #a0745a);
  }

  .leaf-turn-name {
    overflow: hidden;
    font-family: var(--font-display, Georgia, serif);
    font-size: 1.05rem;
    font-style: italic;
    font-weight: 440;
    line-height: 1.25;
    color: var(--color-ink-primary, #34251c);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .leaf-turn-card:hover .leaf-turn-name,
  .leaf-turn-card:focus-visible .leaf-turn-name {
    color: var(--color-ember, #c65f3c);
  }

  @media (max-width: 560px) {
    .leaf-turn {
      grid-template-columns: 1fr;
      gap: 14px;
    }
    .leaf-turn-card--next {
      justify-content: flex-start;
      text-align: left;
      flex-direction: row-reverse;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .leaf-turn-mat,
    .leaf-turn-card--next .leaf-turn-mat,
    .leaf-turn-card:hover .leaf-turn-mat,
    .leaf-turn-card:focus-visible .leaf-turn-mat {
      transform: none;
      transition: none;
    }
  }
</style>
