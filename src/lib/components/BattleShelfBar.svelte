<script lang="ts">
  // Панель над полкой: слово, чьи карты, раса, ранг, тип и порядок.
  //
  // Это не витринные фильтры. Чипы — слова в строку, выбранное отмечено цветом
  // и чертой, как комнаты в `BattleDoor`; заливки и счётчиков «осталось» нет.
  // Печатается только то, что на полке есть: чип расы, у которой нет ни одной
  // карты, отвечал бы «ничего» и учил не нажимать.
  //
  // Две строки, и они разные по службе. Верхняя («полоса») — то, чем пользуются
  // всё время: слово, чьи карты, порядок, счёт. Она прилипает под дверью, пока
  // полка прокручивается. Нижняя («грани») — раса, ранг, тип: их выбирают
  // раз, у начала полки, и прилипшей она отняла бы у карт ещё полсотни точек.
  // Панель кладёт обе прямо в страницу, без общей обёртки: прилипает элемент
  // только внутри своего родителя, а родитель полосы должен быть всей полкой.
  import { t, lang } from '$lib/i18n';
  import { frameFor, frameName, kindLabelKey } from '$lib/battles';
  import type { BattleFrame, BattleCardKind } from '$lib/types/api';
  import {
    SHELF_DEFAULT,
    shelfNarrowed,
    type ShelfFacets,
    type ShelfQuery,
    type ShelfScope,
    type ShelfSort,
  } from '$lib/shelf';

  let {
    query = $bindable(),
    facets,
    frames,
    signedIn,
    total,
    shown,
    mine,
  }: {
    query: ShelfQuery;
    facets: ShelfFacets;
    frames: BattleFrame[];
    /** Кошелёк прочитан: «мои» и «можно взять» есть, чем отвечать. */
    signedIn: boolean;
    total: number;
    shown: number;
    mine: number;
  } = $props();

  const SORTS: { id: ShelfSort; key: 'battlesShelfSortRank' | 'battlesShelfSortKeeper' | 'battlesShelfSortPrice' | 'battlesShelfSortName' }[] = [
    { id: 'tier', key: 'battlesShelfSortRank' },
    { id: 'keeper', key: 'battlesShelfSortKeeper' },
    { id: 'price', key: 'battlesShelfSortPrice' },
    { id: 'name', key: 'battlesShelfSortName' },
  ];

  const SCOPES: { id: ShelfScope; key: 'battlesShelfScopeAll' | 'battlesShelfScopeMine' | 'battlesShelfScopeCan' }[] = [
    { id: 'all', key: 'battlesShelfScopeAll' },
    { id: 'mine', key: 'battlesShelfScopeMine' },
    { id: 'can', key: 'battlesShelfScopeCan' },
  ];

  let narrowed = $derived(shelfNarrowed(query));
  /** Грани на телефоне свёрнуты за кнопку: три строки чипов съедали экран до
   *  первой карты. На столе они на виду всегда. */
  let facetsOpen = $state(false);
  let facetsChosen = $derived(
    [query.raceId, query.tier, query.kind].filter((v) => v !== null).length,
  );
  let hasFacets = $derived(
    facets.races.length > 1 || facets.tiers.length > 1 || facets.kinds.length > 1,
  );

  /** Нажатие на выбранное снимает выбор: «любая» отдельным чипом не нужна. */
  function pickRace(id: string) {
    query.raceId = query.raceId === id ? null : id;
  }
  function pickTier(n: number) {
    query.tier = query.tier === n ? null : n;
  }
  function pickKind(kind: BattleCardKind) {
    query.kind = query.kind === kind ? null : kind;
  }
  function reset() {
    query = { ...SHELF_DEFAULT, sort: query.sort };
  }
</script>

<div class="strip" role="search" aria-label={$t('battlesShelfBarLabel')}>
  <label class="search">
    <span class="sr">{$t('battlesShelfSearch')}</span>
    <input
      type="search"
      bind:value={query.text}
      placeholder={$t('battlesShelfSearch')}
      autocomplete="off"
      spellcheck="false"
    />
  </label>

  {#if signedIn}
    <div class="chips" role="group" aria-label={$t('battlesShelfScopeLabel')}>
      {#each SCOPES as s (s.id)}
        <button
          type="button"
          class="chip"
          class:chip--on={query.scope === s.id}
          aria-pressed={query.scope === s.id}
          onclick={() => (query.scope = s.id)}>{$t(s.key)}</button
        >
      {/each}
    </div>
  {/if}

  <p class="count" aria-live="polite">
    {#if narrowed}
      {$t('battlesShelfShown').replace('{shown}', String(shown)).replace('{total}', String(total))}
      <button type="button" class="reset" onclick={reset}>{$t('battlesShelfReset')}</button>
    {:else}
      {$t('battlesShelfTotal').replace('{n}', String(total))}{#if signedIn && mine > 0}{$t('battlesShelfOwned').replace('{n}', String(mine))}{/if}
    {/if}
  </p>

  <label class="sort">
    <span class="group-name">{$t('battlesShelfSortLabel')}</span>
    <select bind:value={query.sort}>
      {#each SORTS as s (s.id)}
        <option value={s.id}>{$t(s.key)}</option>
      {/each}
    </select>
  </label>
</div>

{#if hasFacets}
  <div class="facets">
    <button
      type="button"
      class="facets-toggle"
      aria-expanded={facetsOpen}
      onclick={() => (facetsOpen = !facetsOpen)}
      >{$t('battlesShelfFilters')}{facetsChosen ? ` · ${facetsChosen}` : ''}</button
    >
    <div class="facets-body" class:facets-body--open={facetsOpen}>
    {#if facets.races.length > 1}
      <div class="group" role="group" aria-label={$t('battlesShelfRace')}>
        <span class="group-name">{$t('battlesShelfRace')}</span>
        <div class="chips">
          {#each facets.races as race (race.id)}
            <button
              type="button"
              class="chip"
              class:chip--on={query.raceId === race.id}
              aria-pressed={query.raceId === race.id}
              onclick={() => pickRace(race.id)}>{race.name}</button
            >
          {/each}
        </div>
      </div>
    {/if}

    {#if facets.tiers.length > 1}
      <div class="group" role="group" aria-label={$t('battlesShelfRank')}>
        <span class="group-name">{$t('battlesShelfRank')}</span>
        <div class="chips">
          {#each facets.tiers as tier (tier)}
            <button
              type="button"
              class="chip"
              class:chip--on={query.tier === tier}
              aria-pressed={query.tier === tier}
              onclick={() => pickTier(tier)}>{frameName(frameFor(tier, frames), $lang)}</button
            >
          {/each}
        </div>
      </div>
    {/if}

    {#if facets.kinds.length > 1}
      <div class="group" role="group" aria-label={$t('battlesShelfKind')}>
        <span class="group-name">{$t('battlesShelfKind')}</span>
        <div class="chips">
          {#each facets.kinds as kind (kind)}
            <button
              type="button"
              class="chip"
              class:chip--on={query.kind === kind}
              aria-pressed={query.kind === kind}
              onclick={() => pickKind(kind)}>{$t(kindLabelKey(kind))}</button
            >
          {/each}
        </div>
      </div>
    {/if}
    </div>
  </div>
{/if}

<style>
  /* Прилипает под дверью. Высоту двери называет страница (`--door-h`): она
     меняется с шириной, и угаданное число съело бы полосу либо оставило щель. */
  .strip {
    position: sticky;
    top: calc(54px + var(--door-h, 2.6rem));
    z-index: 30;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 1.6rem;
    margin: 1.6rem -1.5rem 0;
    padding: 0.5rem 1.5rem;
    background: rgba(248, 241, 231, 0.94);
    backdrop-filter: blur(6px);
    border-top: 1px solid #d8c6b1;
    border-bottom: 1px solid #d8c6b1;
  }

  /* Телефон: дверь там в две строки, и две прилипшие полосы отняли бы треть
     экрана у карт. Полоса идёт вместе со страницей. */
  @media (max-width: 640px) {
    .strip {
      position: static;
    }
  }

  .search {
    flex: 1 1 12rem;
    max-width: 18rem;
  }

  .search input {
    width: 100%;
    padding: 0.4rem 0;
    font: inherit;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1rem;
    color: #34251c;
    background: transparent;
    border: none;
    border-bottom: 1px solid #d8c6b1;
    border-radius: 0;
    outline: none;
  }

  .search input::placeholder {
    font-style: italic;
    color: #8a6a55;
  }

  .search input:focus {
    border-bottom-color: #6f3b24;
  }

  .sort {
    display: inline-flex;
    align-items: baseline;
    gap: 0.6rem;
    margin-left: auto;
  }

  .sort select {
    padding: 0.2rem 0.2rem 0.2rem 0;
    font: inherit;
    font-size: 0.78rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #34251c;
    background: transparent;
    border: none;
    border-bottom: 1px solid #d8c6b1;
    border-radius: 0;
    cursor: pointer;
  }

  .facets {
    border-bottom: 1px solid #d8c6b1;
  }

  .facets-body {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem 2.2rem;
    padding: 0.7rem 0 0.8rem;
  }

  .facets-toggle {
    display: none;
    padding: 0;
    font: inherit;
    font-size: 0.74rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: none;
    cursor: pointer;
  }

  .facets-toggle::after {
    content: '';
    display: inline-block;
    width: 0.4rem;
    height: 0.4rem;
    margin-left: 0.6rem;
    border-right: 1px solid currentColor;
    border-bottom: 1px solid currentColor;
    transform: translateY(-0.15rem) rotate(45deg);
    transition: transform 200ms ease;
  }

  .facets-toggle[aria-expanded='true']::after {
    transform: translateY(0.05rem) rotate(-135deg);
  }

  .facets-toggle:focus-visible {
    outline: 1px solid #6f3b24;
    outline-offset: 3px;
  }

  .group {
    display: flex;
    align-items: baseline;
    gap: 0.3rem 0.9rem;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.1rem 1.1rem;
  }

  .group-name {
    flex: none;
    font-size: 0.62rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .chip {
    padding: 0.2rem 0;
    font: inherit;
    font-size: 0.74rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #5f4636;
    background: none;
    border: none;
    border-bottom: 1px solid transparent;
    cursor: pointer;
  }

  .chip:hover {
    color: #34251c;
    border-bottom-color: #d8c6b1;
  }

  .chip--on {
    color: #6f3b24;
    border-bottom-color: #c65f3c;
  }

  .chip:focus-visible,
  .reset:focus-visible,
  .sort select:focus-visible {
    outline: 1px solid #6f3b24;
    outline-offset: 3px;
  }

  .count {
    display: flex;
    align-items: baseline;
    gap: 0.9rem;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.85rem;
    font-style: italic;
    color: #6f5543;
  }

  .reset {
    padding: 0;
    font: inherit;
    font-size: 0.7rem;
    font-style: normal;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.35);
    cursor: pointer;
  }

  /* Палец, а не мышь: цель нажатия не меньше сорока точек. */
  @media (pointer: coarse), (max-width: 640px) {
    .chip,
    .reset {
      min-height: 2.5rem;
      padding-top: 0.55rem;
      padding-bottom: 0.55rem;
    }

    .search input,
    .sort select {
      min-height: 2.5rem;
    }

    .chip {
      min-width: 2.5rem;
    }

    .facets-toggle {
      display: block;
      min-height: 2.5rem;
    }

    .facets-body {
      display: none;
      gap: 0 1.6rem;
      padding-top: 0;
    }

    .facets-body--open {
      display: flex;
    }

    .group {
      flex-wrap: wrap;
    }
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
