<script lang="ts">
  // Дорога этюдов для телефона: те же бои, что на карте, в одну колонку
  // сверху вниз.
  //
  // Карта на 375 px показывает треть себя, и из девяти боёв видно четыре;
  // сжатая до ширины экрана, она печатала бы подписи в шесть точек. Поэтому
  // уже 760 px страница ставит дорогу вместо карты, а не под ней: один выбор,
  // показанный дважды, — два ответа на один вопрос.
  //
  // С карты взято всё, что несёт смысл, и ничего из местности: порядок боёв,
  // дорога с пройденной частью, знак рода, номер, печать победы, кольцо
  // следующего боя. Лист этюда раскрывается прямо в строке — что в нём,
  // решает страница (`sheet`), тот же сниппет, что у карты.
  import type { Snippet } from 'svelte';
  import { slide } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { roman } from '$lib/etudeMap';
  import type { BattleChallenge } from '$lib/types/api';
  import EtudeGlyph from './EtudeGlyph.svelte';
  import type { EtudeKind } from './EtudeMap.svelte';

  let {
    challenges,
    kindOf,
    titleOf,
    sheet,
  }: {
    challenges: BattleChallenge[];
    kindOf: (c: BattleChallenge) => EtudeKind;
    titleOf: (c: BattleChallenge) => string;
    sheet: Snippet<[BattleChallenge, boolean]>;
  } = $props();

  let openId = $state<string | null>(null);
  let still = $state(false);

  $effect(() => {
    still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });

  /** Следующий бой — первый не выигранный; правило то же, что у карты. */
  let current = $derived.by(() => {
    const i = challenges.findIndex((c) => !c.marks?.won);
    return i < 0 ? challenges.length : i;
  });
</script>

<ol class="road" aria-label={$t('battleMapLabel')}>
  {#each challenges as c, i (c.id)}
    {@const open = openId === c.id}
    <li
      class="stop"
      class:stop--current={i === current}
      class:stop--won={c.marks?.won}
      class:in-walked={i > 0 && i <= current}
      class:out-walked={i < current}
      class:last={i === challenges.length - 1}
    >
      <button
        type="button"
        class="head"
        aria-expanded={open}
        aria-controls="etude-stop-{c.id}"
        onclick={() => (openId = open ? null : c.id)}
      >
        <span class="mark" aria-hidden="true">
          <svg viewBox="-34 -62 68 70">
            {#if i === current}
              <ellipse class="next-ring" rx="26" ry="7" cy="1" />
              <ellipse class="next-ring next-ring--outer" rx="32" ry="9.5" cy="1" />
            {/if}
            <EtudeGlyph kind={kindOf(c)} />
          </svg>
        </span>
        <span class="words">
          <span class="number">{roman(i + 1)}</span>
          <span class="title">{titleOf(c)}</span>
          {#if i === current}
            <span class="state state--next">{$t('battleMapNext')}</span>
          {:else if c.marks?.won}
            <span class="state">
              <svg class="seal" viewBox="-11 -11 22 22" aria-hidden="true">
                <path
                  class="seal-wax"
                  d="M0 -10l3 1l3 -1l2 3l3 1l0 3l2 3l-2 3l0 3l-3 1l-2 3l-3 -1l-3 1l-3 -1l-3 1l-2 -3l-3 -1l0 -3l-2 -3l2 -3l0 -3l3 -1l2 -3l3 1z"
                />
                <circle class="seal-ring" class:seal-ring--clean={c.marks.clean} r="5.5" />
              </svg>
              {c.marks.clean ? $t('battleMarkClean') : $t('battleMarkWon')}
            </span>
          {/if}
        </span>
        <span class="chevron" aria-hidden="true">{open ? '−' : '+'}</span>
      </button>
      {#if open}
        <div class="sheet" id="etude-stop-{c.id}" transition:slide={{ duration: still ? 0 : 180 }}>
          {@render sheet(c, true)}
        </div>
      {/if}
    </li>
  {/each}
</ol>

<style>
  .road {
    --ink: #3d2a1e;
    --paper: #f3e7cd;
    --rail: 3.6rem;
    margin: 0;
    padding: 0.4rem 0 0;
    list-style: none;
  }

  /* Дорога — две половины у каждой остановки: до знака и после него. Своя
     половина знает, пройдена ли она, поэтому пройденный путь кончается
     ровно на следующем бое, как на карте. */
  .stop {
    position: relative;
  }

  .stop::before,
  .stop::after {
    content: '';
    position: absolute;
    left: calc(var(--rail) / 2 - 1px);
    width: 0;
    border-left: 2px dotted rgba(111, 59, 36, 0.7);
  }

  .stop::before {
    top: 0;
    height: 1.1rem;
  }

  .stop::after {
    top: 3.6rem;
    bottom: 0;
  }

  .stop:first-child::before,
  .stop.last::after {
    display: none;
  }

  .stop.in-walked::before,
  .stop.out-walked::after {
    border-left: 3px dashed #8f3a22;
    left: calc(var(--rail) / 2 - 1.5px);
  }

  .head {
    display: grid;
    grid-template-columns: var(--rail) 1fr auto;
    align-items: center;
    gap: 0 0.6rem;
    width: 100%;
    min-height: 4.4rem;
    padding: 0.5rem 0;
    font: inherit;
    text-align: left;
    color: #34251c;
    background: none;
    border: 0;
    border-bottom: 1px solid rgba(216, 198, 177, 0.7);
    cursor: pointer;
  }

  .stop.last .head {
    border-bottom: 0;
  }

  .mark {
    display: flex;
    justify-content: center;
  }

  .mark svg {
    width: 3.2rem;
    height: 3.3rem;
    overflow: visible;
  }

  .next-ring {
    fill: none;
    stroke: #6f3b24;
    stroke-width: 1.8;
  }

  .next-ring--outer {
    stroke-width: 1;
    stroke-dasharray: 3.5 3;
  }

  .words {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }

  .number {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.16em;
    color: #6f3b24;
  }

  .title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 1.3rem;
    font-style: italic;
    font-weight: 600;
    line-height: 1.15;
  }

  .stop--current .title {
    color: #a8442a;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.72rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .state--next {
    color: #a8442a;
  }

  .seal {
    width: 1rem;
    height: 1rem;
  }

  .seal-wax {
    fill: #a8442a;
    stroke: #6f2a1a;
    stroke-width: 0.8;
  }

  .seal-ring {
    fill: none;
    stroke: rgba(255, 220, 190, 0.6);
    stroke-width: 1;
  }

  .seal-ring--clean {
    stroke: #d9b45a;
    stroke-width: 1.8;
  }

  .chevron {
    width: 1.6rem;
    font-family: Georgia, serif;
    font-size: 1.2rem;
    text-align: center;
    color: #8a6a55;
  }

  .head:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }

  /* Лист этюда стоит справа от дороги, чтобы дорога шла мимо него дальше. */
  .sheet {
    padding: 0.4rem 0 1rem calc(var(--rail) + 0.6rem);
    border-bottom: 1px solid rgba(216, 198, 177, 0.7);
  }
</style>
