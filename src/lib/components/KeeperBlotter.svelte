<script lang="ts">
  /**
   * The keeper's writing line and the handful of plates it returns.
   * Used in the home frame and in the header panel — same query, same voice.
   */
  import { fade } from 'svelte/transition';
  import { tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import { figurineHref } from '$lib/figurineHref';
  import { keeper } from '$lib/stores/keeper.svelte';
  import {
    assembleResults,
    KEEPER_BLOTTER_MAX,
    localMatch,
    matchKind,
    whisperSeeds,
    type MatchKind,
  } from '$lib/keeper-search';
  import type { FigurineListItem } from '$lib/types/api';

  type Props = {
    autofocus?: boolean;
    /** Analytics source on plates that leave the hall. */
    source?: string;
  };

  let { autofocus = false, source = 'home_keeper' }: Props = $props();

  let inputEl = $state<HTMLInputElement | null>(null);

  let figById = $derived(new Map(keeper.figurines.map((f) => [f.id, f])));
  let allowed = $derived(new Set(keeper.figurines.map((f) => f.id)));
  let reelSet = $derived(new Set(keeper.reelIds));
  let whispers = $derived(whisperSeeds(keeper.figurines, $lang));
  let localHits = $derived(
    keeper.query.trim().length < 2 ? [] : localMatch(keeper.figurines, keeper.query),
  );
  let blotter = $derived(
    assembleResults(keeper.hits, localHits, figById, allowed, KEEPER_BLOTTER_MAX).slice(
      0,
      KEEPER_BLOTTER_MAX,
    ),
  );
  let asked = $derived(keeper.query.trim().length >= 2);
  let empty = $derived(asked && !keeper.loading && blotter.length === 0);
  let archiveHref = $derived(
    asked ? `/figurines?q=${encodeURIComponent(keeper.query.trim())}` : '/figurines',
  );

  $effect(() => {
    const seq = keeper.focusSeq;
    if (!autofocus || seq === 0) return;
    void tick().then(() => inputEl?.focus());
  });

  function reasonOf(fig: FigurineListItem): string {
    const kind: MatchKind = matchKind(keeper.query, fig);
    if (kind === 'inHouse') return $t('homeKeeperReasonInHouse');
    if (kind === 'name') return $t('homeKeeperReasonName');
    if (kind === 'series' && fig.series?.trim()) return fig.series.trim();
    if (kind === 'craft') {
      const bit = [fig.material, fig.technique, fig.dimensions].map((v) => v?.trim()).find(Boolean);
      return bit || $t('homeKeeperReasonCraft');
    }
    return $t('homeKeeperReasonDescription');
  }

  function openPlate(fig: FigurineListItem) {
    keeper.closePanel();
    if (reelSet.has(fig.id)) {
      document.getElementById(`work-${fig.id}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      return;
    }
    void goto(figurineHref(fig, source));
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      keeper.runNow();
    }
  }
</script>

<div class="blotter">
  <label class="kn-line">
    <span class="sr-only">{$t('homeKeeperTitle')}</span>
    <input
      bind:this={inputEl}
      type="search"
      id="keeper-query-{source}"
      name="q"
      class="kn-input"
      value={keeper.query}
      placeholder={$t('homeKeeperPlaceholder')}
      autocomplete="off"
      spellcheck="false"
      oninput={(e) => keeper.setQuery(e.currentTarget.value)}
      onkeydown={onKey}
    />
    {#if keeper.query}
      <button
        type="button"
        class="kn-clear"
        onclick={() => keeper.setQuery('')}
        aria-label={$t('archiveClearFilters')}
      >
        <svg width="9" height="9" viewBox="0 0 9 9" fill="none" aria-hidden="true">
          <path d="M1 1l7 7M8 1L1 8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
      </button>
    {/if}
  </label>

  {#if !asked}
    <ul class="kn-whispers">
      {#each whispers as w}
        <li>
          <button type="button" class="kn-whisper" onclick={() => keeper.setQuery(w)}>{w}</button>
        </li>
      {/each}
    </ul>
  {:else if keeper.loading && blotter.length === 0}
    <p class="kn-status" in:fade={{ duration: 400 }}>{$t('homeKeeperThinking')}</p>
  {:else if keeper.error && blotter.length === 0}
    <p class="kn-status" in:fade={{ duration: 400 }}>{$t('homeKeeperError')}</p>
  {:else if empty}
    <div class="kn-empty" in:fade={{ duration: 500 }}>
      <p class="kn-empty-title">{$t('homeKeeperEmpty')}</p>
      <a class="kn-ledger" href={archiveHref} onclick={() => keeper.closePanel()}>{$t('homeKeeperLedger')}</a>
    </div>
  {:else if blotter.length > 0}
    <ul class="kn-plates" in:fade={{ duration: 500 }}>
      {#each blotter as fig (fig.id)}
        <li>
          <button type="button" class="kn-plate" onclick={() => openPlate(fig)}>
            <span class="kn-thumb">
              <AppImage
                src={fig.thumbUrl ?? fig.faceImageUrl}
                thumbUrl={fig.thumbUrl}
                alt=""
                class="kn-img"
                sizes="72px"
              />
            </span>
            <span class="kn-meta">
              <span class="kn-name">{fig.name}</span>
              <span class="kn-why">{reasonOf(fig)}</span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
    <a class="kn-ledger" href={archiveHref} onclick={() => keeper.closePanel()}>{$t('homeKeeperLedger')}</a>
  {/if}
</div>

<style>
  .kn-line {
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid color-mix(in srgb, #34251c 28%, transparent);
    padding-bottom: 6px;
  }

  .kn-input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    font-family: 'Georgia', serif;
    font-size: 1.05rem;
    font-style: italic;
    color: #34251c;
    outline: none;
    padding: 4px 0;
  }
  .kn-input::placeholder {
    color: color-mix(in srgb, #7c6554 70%, transparent);
  }
  .kn-input::-webkit-search-cancel-button { -webkit-appearance: none; }

  .kn-clear {
    flex: 0 0 auto;
    width: 22px;
    height: 22px;
    border: 0;
    background: transparent;
    color: #7c6554;
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .kn-clear:hover { color: #34251c; }

  .kn-whispers {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
  }
  .kn-whisper {
    border: 0;
    background: none;
    padding: 0;
    font-family: 'Georgia', serif;
    font-style: italic;
    font-size: 0.88rem;
    color: color-mix(in srgb, #7c6554 82%, transparent);
    cursor: pointer;
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 3px;
    text-decoration-color: color-mix(in srgb, #d8c6b1 80%, transparent);
  }
  .kn-whisper:hover { color: #6f3b24; }

  .kn-status {
    margin: 18px 0 0;
    font-family: 'Georgia', serif;
    font-style: italic;
    font-size: 0.92rem;
    color: #7c6554;
  }

  .kn-empty { margin-top: 18px; }
  .kn-empty-title {
    margin: 0 0 10px;
    font-family: 'Fraunces', serif;
    font-size: 1.15rem;
    color: #6f3b24;
  }

  .kn-plates {
    list-style: none;
    margin: 18px 0 0;
    padding: 0;
    display: flex;
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 4px;
    scrollbar-width: thin;
  }
  .kn-plate {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: min(220px, 78vw);
    max-width: 280px;
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: inherit;
  }
  .kn-thumb {
    position: relative;
    flex: 0 0 56px;
    width: 56px;
    height: 72px;
    overflow: hidden;
    background: #ede3cf;
    border: 1px solid color-mix(in srgb, #34251c 14%, transparent);
  }
  .kn-thumb :global(.app-image-wrap) {
    width: 100%;
    height: 100%;
  }
  .kn-meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .kn-name {
    font-family: 'Fraunces', serif;
    font-size: 0.95rem;
    color: #34251c;
    line-height: 1.25;
  }
  .kn-why {
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #7c6554;
  }

  .kn-ledger {
    display: inline-flex;
    margin-top: 16px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #6f3b24;
    text-decoration: none;
    border-bottom: 1px solid color-mix(in srgb, #6f3b24 28%, transparent);
    padding-bottom: 3px;
  }
  .kn-ledger:hover { color: #c65f3c; }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (min-width: 760px) {
    .kn-plates {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
      overflow: visible;
    }
    .kn-plate { min-width: 0; max-width: none; }
  }
</style>
