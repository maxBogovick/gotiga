<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { FigurineListItem } from '$lib/types/api';
  import { figurineHref } from '$lib/figurineHref';
  import { t } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import { pageTurn } from '$lib/stores/page-turn.svelte';
  import { turnSound } from '$lib/stores/page-turn-sound.svelte';
  import { playTurnSound } from '$lib/audio/page-turn-sounds';
  import { detailHeader } from '$lib/stores/detail-header.svelte';
  import { houseClock } from '$lib/stores/house-clock.svelte';
  import { showingRooms } from '$lib/stores/showing-rooms.svelte';
  import { isGated, isShowingOpen, resolveWindow } from '$lib/showing-window';

  let {
    side,
    work = null,
  }: {
    side: 'prev' | 'next';
    work?: FigurineListItem | null;
  } = $props();

  let sealed = $derived.by(() => {
    if (!work) return false;
    const win = resolveWindow(work, showingRooms.list);
    return isGated(win) && !isShowingOpen(win, houseClock.nowDate);
  });

  let href = $derived(work ? figurineHref(work) : '');
  let label = $derived(
    work
      ? `${side === 'prev' ? $t('figurineNavPrev') : $t('figurineNavNext')}: ${work.name}`
      : ''
  );

  function peekOn() {
    if (work) detailHeader.setPeek(side);
  }
  function peekOff() {
    if (detailHeader.peek === side) detailHeader.setPeek(null);
  }

  function armPageTurn(e: MouseEvent) {
    if (!work) return;
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if (typeof window !== 'undefined' && !window.matchMedia('(pointer: fine)').matches) return;
    const direction = side === 'prev' ? 'backward' : 'forward';
    pageTurn.arm(direction);
    const sound = turnSound.value;
    if (sound !== 'off') playTurnSound(sound, direction);
  }

  onDestroy(() => {
    if (detailHeader.peek === side) detailHeader.setPeek(null);
  });
</script>

{#snippet chevron()}
  <svg class="neighbor-chevron" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true">
    {#if side === 'prev'}
      <path d="M6.5 2L3.5 5 6.5 8" stroke-linecap="round" stroke-linejoin="round"/>
    {:else}
      <path d="M3.5 2l3 3L3.5 8" stroke-linecap="round" stroke-linejoin="round"/>
    {/if}
  </svg>
{/snippet}

{#snippet mat()}
  <span class="neighbor-frame">
    <span class="neighbor-mat">
      {#if work && !sealed}
        <AppImage
          src={work.faceImageUrl}
          thumbUrl={work.thumbUrl}
          alt=""
          class="neighbor-img"
          sizes="48px"
          width={48}
          height={48}
          loading="lazy"
        />
      {:else if work && sealed}
        <span class="neighbor-seal" aria-hidden="true"></span>
      {/if}
    </span>
    {#if work}
      <span class="neighbor-whisper">{work.name}</span>
    {/if}
  </span>
{/snippet}

{#if work}
  <a
    href={href}
    class="neighbor-plate neighbor-plate--{side}"
    class:is-sealed={sealed}
    title={work.name}
    aria-label={label}
    data-sveltekit-preload-data="hover"
    onclick={armPageTurn}
    onpointerenter={peekOn}
    onpointerleave={peekOff}
    onfocus={peekOn}
    onblur={peekOff}
  >
    {#if side === 'prev'}{@render chevron()}{/if}
    {@render mat()}
    {#if side === 'next'}{@render chevron()}{/if}
  </a>
{:else}
  <span class="neighbor-plate neighbor-plate--{side} neighbor-plate--empty" aria-hidden="true">
    {#if side === 'prev'}{@render chevron()}{/if}
    {@render mat()}
    {#if side === 'next'}{@render chevron()}{/if}
  </span>
{/if}

<style>
  .neighbor-plate {
    --plate-size: 44px;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    flex: 0 0 auto;
    height: var(--plate-size);
    text-decoration: none;
    color: inherit;
    cursor: pointer;
  }

  :global(.site-header.is-scrolled) .neighbor-plate {
    --plate-size: 32px;
  }

  :global(.site-header.is-leaf) .neighbor-plate,
  :global(.site-header.is-leaf.is-scrolled) .neighbor-plate {
    --plate-size: 40px;
  }

  /* Idle twitch: a small lift every few seconds so the plates read as
     something you can take, not ornaments. Prev/next are out of phase.
     Hover pauses so the lift doesn't fight the pointer. */
  .neighbor-plate:not(.neighbor-plate--empty) {
    animation: neighbor-twitch 3.8s cubic-bezier(0.16, 1, 0.3, 1) infinite;
  }

  .neighbor-plate--next:not(.neighbor-plate--empty) {
    animation-delay: 1.6s;
  }

  .neighbor-plate:not(.neighbor-plate--empty):hover,
  .neighbor-plate:not(.neighbor-plate--empty):focus-visible {
    animation-play-state: paused;
  }

  @keyframes neighbor-twitch {
    0%, 14%, 100% { transform: translateY(0); }
    5% { transform: translateY(-4px); }
    9% { transform: translateY(1px); }
  }

  .neighbor-chevron {
    flex: 0 0 auto;
    display: block;
    color: color-mix(in srgb, var(--copper, #c65f3c) 78%, var(--ink, #34251c));
    opacity: 0.7;
    transition: transform 0.28s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.2s ease, color 0.2s ease;
  }

  .neighbor-plate:hover .neighbor-chevron,
  .neighbor-plate:focus-visible .neighbor-chevron {
    opacity: 1;
    color: var(--copper, #c65f3c);
  }

  .neighbor-plate--prev:hover .neighbor-chevron,
  .neighbor-plate--prev:focus-visible .neighbor-chevron {
    transform: translateX(-2px);
  }

  .neighbor-plate--next:hover .neighbor-chevron,
  .neighbor-plate--next:focus-visible .neighbor-chevron {
    transform: translateX(2px);
  }

  .neighbor-frame {
    position: relative;
    flex: 0 0 auto;
    width: var(--plate-size);
    height: var(--plate-size);
  }

  .neighbor-mat {
    position: relative;
    display: block;
    width: 100%;
    height: 100%;
    overflow: hidden;
    border-radius: 2px;
    background: color-mix(in srgb, var(--color-canvas-sunken, #ede3cf) 70%, transparent);
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--copper, #c65f3c) 55%, transparent),
      0 0 0 3px color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 92%, white),
      0 0 0 4px color-mix(in srgb, var(--copper, #c65f3c) 28%, transparent);
    transform: rotate(-1deg);
    transition: transform 0.28s cubic-bezier(0.16, 1, 0.3, 1), box-shadow 0.28s ease;
  }

  .neighbor-plate--next .neighbor-mat {
    transform: rotate(1deg);
  }

  .neighbor-plate:not(.neighbor-plate--empty):hover .neighbor-mat,
  .neighbor-plate:not(.neighbor-plate--empty):focus-visible .neighbor-mat {
    transform: rotate(0deg) scale(1.08);
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--copper, #c65f3c) 72%, transparent),
      0 0 0 3px color-mix(in srgb, var(--color-canvas-base, #f8f1e7) 92%, white),
      0 0 0 4px color-mix(in srgb, var(--copper, #c65f3c) 42%, transparent),
      0 6px 16px color-mix(in srgb, var(--ink, #34251c) 18%, transparent);
  }

  .neighbor-plate:focus-visible {
    outline: none;
  }

  .neighbor-plate--empty {
    pointer-events: none;
    cursor: default;
  }

  .neighbor-plate--empty .neighbor-chevron {
    visibility: hidden;
  }

  .neighbor-plate--empty .neighbor-mat {
    background: transparent;
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--ink, #34251c) 12%, transparent),
      0 0 0 3px transparent,
      0 0 0 4px color-mix(in srgb, var(--ink, #34251c) 7%, transparent);
    opacity: 0.45;
  }

  .neighbor-img {
    width: 100%;
    height: 100%;
  }

  .neighbor-img :global(.app-image-main) {
    object-fit: cover;
  }

  .neighbor-seal {
    display: block;
    width: 100%;
    height: 100%;
    background:
      radial-gradient(ellipse at 50% 42%, color-mix(in srgb, var(--ink, #34251c) 18%, transparent), transparent 62%),
      color-mix(in srgb, var(--ink, #34251c) 72%, #6f3b24);
  }

  .neighbor-whisper {
    position: absolute;
    top: calc(100% + 8px);
    z-index: 6;
    width: max-content;
    max-width: min(16rem, 56vw);
    color: color-mix(in srgb, var(--ink, #34251c) 78%, transparent);
    font-family: var(--font-serif, Georgia, serif);
    font-size: 0.78rem;
    font-style: italic;
    font-weight: 400;
    letter-spacing: 0;
    line-height: 1.3;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.18s ease;
  }

  .neighbor-plate--prev .neighbor-whisper {
    left: 0;
  }

  .neighbor-plate--next .neighbor-whisper {
    right: 0;
    text-align: right;
  }

  .neighbor-plate:hover .neighbor-whisper,
  .neighbor-plate:focus-visible .neighbor-whisper {
    opacity: 1;
  }

  :global(.site-header.is-scrolled) .neighbor-whisper {
    display: none;
  }

  :global(.site-header.is-leaf.is-scrolled) .neighbor-whisper {
    display: block;
  }

  @media (max-width: 720px) {
    .neighbor-plate { display: none; }
  }

  @media (prefers-reduced-motion: reduce) {
    .neighbor-plate:not(.neighbor-plate--empty) {
      animation: none;
    }
    .neighbor-mat,
    .neighbor-plate--next .neighbor-mat {
      transform: none;
      transition: none;
    }
    .neighbor-plate:not(.neighbor-plate--empty):hover .neighbor-mat,
    .neighbor-plate:not(.neighbor-plate--empty):focus-visible .neighbor-mat {
      transform: none;
    }
    .neighbor-plate--prev:hover .neighbor-chevron,
    .neighbor-plate--prev:focus-visible .neighbor-chevron,
    .neighbor-plate--next:hover .neighbor-chevron,
    .neighbor-plate--next:focus-visible .neighbor-chevron {
      transform: none;
    }
  }
</style>
