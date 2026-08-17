<script lang="ts">
  /**
   * A blotter on the home collection: the visitor writes a note to the keeper.
   * The reel below is not filtered. Results are a handful of plates, not a shop grid.
   * No view-transition-name on thumbs — a duplicate figurine-{id} would abort the morph.
   */
  import { fade } from 'svelte/transition';
  import { goto } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import AppImage from '$lib/components/AppImage.svelte';
  import { figurineHref } from '$lib/figurineHref';
  import type { FigurineListItem, SemanticHit } from '$lib/types/api';
  import {
    assembleResults,
    isInHouseQuery,
    KEEPER_BLOTTER_MAX,
    localMatch,
    matchKind,
    whisperSeeds,
    type MatchKind,
  } from '$lib/keeper-search';

  type Props = {
    figurines: FigurineListItem[];
    /** Works currently on the home reel — those plates scroll the hall, the rest leave it. */
    reelIds: string[];
  };

  let { figurines, reelIds }: Props = $props();

  let query = $state('');
  let keeperHits = $state<SemanticHit[] | null>(null);
  let keeperLoading = $state(false);
  let keeperError = $state(false);
  let keeperSeq = 0;
  let searchDebounce: ReturnType<typeof setTimeout> | undefined;

  let figById = $derived(new Map(figurines.map((f) => [f.id, f])));
  let allowed = $derived(new Set(figurines.map((f) => f.id)));
  let reelSet = $derived(new Set(reelIds));
  let whispers = $derived(whisperSeeds(figurines, $lang));
  let localHits = $derived(query.trim().length < 2 ? [] : localMatch(figurines, query));
  let blotter = $derived(
    assembleResults(keeperHits, localHits, figById, allowed, KEEPER_BLOTTER_MAX).slice(
      0,
      KEEPER_BLOTTER_MAX,
    ),
  );
  let asked = $derived(query.trim().length >= 2);
  let empty = $derived(asked && !keeperLoading && blotter.length === 0);
  let archiveHref = $derived(
    asked ? `/figurines?q=${encodeURIComponent(query.trim())}` : '/figurines',
  );

  $effect(() => {
    const q = query.trim();
    clearTimeout(searchDebounce);
    if (q.length < 2 || isInHouseQuery(q)) {
      keeperHits = null;
      keeperError = false;
      keeperSeq++;
      return;
    }
    searchDebounce = setTimeout(() => runKeeper(q), 350);
    return () => clearTimeout(searchDebounce);
  });

  async function runKeeper(q: string) {
    const seq = ++keeperSeq;
    keeperLoading = true;
    keeperError = false;
    try {
      const hits = await api.semanticSearch(q);
      if (seq !== keeperSeq) return;
      keeperHits = hits;
    } catch {
      if (seq !== keeperSeq) return;
      keeperError = true;
      keeperHits = null;
    } finally {
      if (seq === keeperSeq) keeperLoading = false;
    }
  }

  function reasonOf(fig: FigurineListItem): string {
    const kind: MatchKind = matchKind(query, fig);
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
    if (reelSet.has(fig.id)) {
      document.getElementById(`work-${fig.id}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      return;
    }
    void goto(figurineHref(fig, 'home_keeper'));
  }

  function applyWhisper(w: string) {
    query = w;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      clearTimeout(searchDebounce);
      const q = query.trim();
      if (q.length >= 2) void runKeeper(q);
    }
  }
</script>

{#if figurines.length > 0}
<section class="keeper-note" aria-labelledby="keeper-note-title">
  <span class="fc fc-tl"></span>
  <span class="fc fc-tr"></span>
  <span class="fc fc-bl"></span>
  <span class="fc fc-br"></span>

  <p class="kn-eyebrow"><span class="kn-rule"></span>{$t('homeKeeperEyebrow')}</p>
  <h3 id="keeper-note-title" class="kn-title">{$t('homeKeeperTitle')}</h3>
  <p class="kn-lead">{$t('homeKeeperLead')}</p>

  <label class="kn-line">
    <span class="sr-only">{$t('homeKeeperTitle')}</span>
    <input
      type="search"
      class="kn-input"
      bind:value={query}
      placeholder={$t('homeKeeperPlaceholder')}
      autocomplete="off"
      spellcheck="false"
      onkeydown={onKey}
    />
    {#if query}
      <button type="button" class="kn-clear" onclick={() => (query = '')} aria-label={$t('archiveClearFilters')}>
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
          <button type="button" class="kn-whisper" onclick={() => applyWhisper(w)}>{w}</button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if asked}
    {#if keeperLoading && blotter.length === 0}
      <p class="kn-status" in:fade={{ duration: 400 }}>{$t('homeKeeperThinking')}</p>
    {:else if keeperError && blotter.length === 0}
      <p class="kn-status" in:fade={{ duration: 400 }}>{$t('homeKeeperError')}</p>
    {:else if empty}
      <div class="kn-empty" in:fade={{ duration: 500 }}>
        <p class="kn-empty-title">{$t('homeKeeperEmpty')}</p>
        <a class="kn-ledger" href={archiveHref}>{$t('homeKeeperLedger')}</a>
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
      <a class="kn-ledger" href={archiveHref}>{$t('homeKeeperLedger')}</a>
    {/if}
  {/if}
</section>
{/if}

<style>
  .keeper-note {
    position: relative;
    width: 100%;
    max-width: var(--reel-card-width, 64rem);
    box-sizing: border-box;
    margin: clamp(1.25rem, 3vw, 2.25rem) auto 0;
    padding: clamp(22px, 3vw, 36px) clamp(20px, 3.2vw, 40px) clamp(20px, 2.6vw, 32px);
    background: color-mix(in srgb, var(--color-canvas-raised, #f2e8d8) 88%, #fff 12%);
    border: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-ink-primary, #34251c) 6%, transparent);
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
