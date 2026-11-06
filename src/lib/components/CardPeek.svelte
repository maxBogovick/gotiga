<script module lang="ts">
  import type { BattleCard } from '$lib/types/api';

  export type PeekHover = {
    card: BattleCard;
    left: number;
    top?: number;
    bottom?: number;
    /** Строка под текстом: здоровье, цена и т. п. */
    note?: string;
  };
</script>

<script lang="ts">
  // Подробности карты вне боя: попап при наведении и крупный лист по нажатию.
  // Состояние держит страница (`hover`, `look`): здесь только то, что нарисовать.
  // Лист — единственный путь прочитать текст карты на телефоне, где наведения нет.
  import { t, lang } from '$lib/i18n';
  import { cardCopy } from '$lib/battles';
  import BattleCardFace from '$lib/components/BattleCard.svelte';
  import { interact } from '$lib/analytics';
  import { loadWorld, talesOfWork, workHref } from '$lib/world';
  import { taleHref } from '$lib/tales';
  import type { BattleFrame, GazetteLeaf } from '$lib/types/api';

  let {
    hover = null,
    look = null,
    frames,
    note,
    onclose,
  }: {
    hover?: PeekHover | null;
    look?: BattleCard | null;
    frames: BattleFrame[];
    /** Строка под текстом листа. */
    note?: string;
    onclose: () => void;
  } = $props();

  const stats = (c: BattleCard) => [
    `${$t('battlesPowerLabel')} ${c.power}`,
    `${$t('battleStatArmour')} ${c.armor}`,
    `${$t('battleStatWard')} ${c.ward}`,
    `${$t('battleStatReach')} ${c.reach}`,
  ];

  // Работа, по которой сделана карта, и байка о ней: лист карты ведёт к ним, как и лист полки.
  let tale = $state<GazetteLeaf | null>(null);
  $effect(() => {
    const id = look?.figurineId;
    tale = null;
    if (!id) return;
    let alive = true;
    void loadWorld()
      .then((w) => {
        if (alive) tale = talesOfWork(w, id)[0] ?? null;
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  });

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
  function focusClose(node: HTMLElement) {
    node.focus();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && look && onclose()} />

{#if hover && !look}
  {@const c = hover.card}
  {@const copy = cardCopy(c, $lang)}
  <div
    use:portal
    class="peek-pop"
    style:left="{hover.left}px"
    style:top={hover.top !== undefined ? `${hover.top}px` : undefined}
    style:bottom={hover.bottom !== undefined ? `${hover.bottom}px` : undefined}
    aria-hidden="true"
  >
    <p class="peek-title">{copy.title}</p>
    <ul class="peek-stats">
      {#each stats(c) as st (st)}<li>{st}</li>{/each}
    </ul>
    {#if copy.effect}<p class="peek-text">{copy.effect}</p>{/if}
    {#if hover.note}<p class="peek-note">{hover.note}</p>{/if}
  </div>
{/if}

{#if look}
  {@const copy = cardCopy(look, $lang)}
  <div use:portal class="peek-root">
    <button type="button" class="peek-scrim" aria-label={$t('runClose')} onclick={onclose} tabindex="-1"></button>
    <div class="peek-sheet" role="dialog" aria-modal="true" aria-label={copy.title}>
      <button type="button" class="peek-x" aria-label={$t('runClose')} use:focusClose onclick={onclose}>×</button>
      <div class="peek-face" aria-hidden="true">
        <BattleCardFace card={look} {frames} owned={true} uniform transition={false} interactive={false} />
      </div>
      <div class="peek-body">
        <h2 class="peek-h">{copy.title}</h2>
        <ul class="peek-stats peek-stats--big">
          {#each stats(look) as st (st)}<li>{st}</li>{/each}
        </ul>
        {#if copy.effect}<p class="peek-text peek-text--big">{copy.effect}</p>{/if}
        {#if copy.lore}<p class="peek-lore">{copy.lore}</p>{/if}
        {#if note}<p class="peek-note">{note}</p>{/if}
        {#if workHref(look) || tale}
          <p class="peek-links">
            {#if workHref(look)}
              <a href={workHref(look)} onclick={() => interact('world_link', 'run_to_work')}>{$t('battlesWorkLink')} →</a>
            {/if}
            {#if tale}
              <a href={taleHref(tale, 'run_card')} onclick={() => interact('world_link', 'run_to_tale')}>{$t('worldCardTale')} →</a>
            {/if}
          </p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .peek-pop {
    position: fixed;
    z-index: 70;
    box-sizing: border-box;
    width: min(260px, calc(100vw - 16px));
    padding: 0.65rem 0.8rem 0.7rem;
    border: 1px solid #d8c6b1;
    border-left: 3px solid #c65f3c;
    background: #fcf7ee;
    color: #34251c;
    box-shadow: 0 10px 28px rgba(52, 37, 28, 0.38), 0 0 0 3px rgba(248, 241, 231, 0.55);
    font-family: Inter, system-ui, sans-serif;
    font-size: 0.85rem;
    line-height: 1.45;
    pointer-events: none;
    animation: peek-in 0.14s ease-out;
  }
  .peek-pop p {
    margin: 0;
  }
  .peek-title {
    margin-bottom: 0.35rem !important;
    font-family: Fraunces, Georgia, serif;
    font-size: 1.1rem;
    font-weight: 600;
    line-height: 1.2;
  }
  .peek-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.35rem;
    margin: 0 0 0.45rem;
    padding: 0;
    list-style: none;
  }
  .peek-stats li {
    padding: 0.05rem 0.45rem;
    border: 1px solid #d8c6b1;
    background: #f1e6d4;
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .peek-text {
    color: #34251c;
  }
  .peek-note {
    margin-top: 0.4rem !important;
    padding-top: 0.4rem;
    border-top: 1px dashed #d8c6b1;
    color: #6f3b24;
  }

  .peek-root {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
  .peek-scrim {
    position: absolute;
    inset: 0;
    border: 0;
    background: rgba(27, 20, 16, 0.55);
    cursor: default;
  }
  .peek-sheet {
    position: relative;
    box-sizing: border-box;
    display: grid;
    gap: 0.9rem;
    justify-items: center;
    width: 100%;
    max-height: 92vh;
    overflow-y: auto;
    padding: 1.1rem 1.1rem 1.4rem;
    border-top: 3px solid #c65f3c;
    background: #f8f1e7;
    color: #34251c;
    box-shadow: 0 -12px 40px rgba(27, 20, 16, 0.5);
    animation: peek-up 0.2s ease-out;
  }
  .peek-x {
    position: absolute;
    top: 0.4rem;
    right: 0.5rem;
    width: 2.4rem;
    height: 2.4rem;
    border: 1px solid #d8c6b1;
    border-radius: 50%;
    background: #fcf7ee;
    color: #6f3b24;
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
  }
  .peek-face {
    width: min(18rem, 68vw);
  }
  .peek-body {
    width: 100%;
    max-width: 28rem;
  }
  .peek-h {
    margin: 0 0 0.5rem;
    font-family: Fraunces, Georgia, serif;
    font-size: 1.4rem;
    line-height: 1.2;
  }
  .peek-stats--big li {
    padding: 0.15rem 0.6rem;
    font-size: 0.9rem;
  }
  .peek-text--big {
    margin: 0 0 0.6rem;
    font-size: 1.05rem;
    line-height: 1.5;
  }
  .peek-lore {
    margin: 0 0 0.6rem;
    color: #5a4535;
    font-style: italic;
    line-height: 1.5;
  }
  .peek-links {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 1.1rem;
    margin: 0.8rem 0 0;
  }
  .peek-links a {
    color: #6f3b24;
    text-underline-offset: 3px;
  }
  .peek-body .peek-note {
    margin: 0.4rem 0 0;
  }
  @media (min-width: 48rem) {
    .peek-root {
      align-items: center;
    }
    .peek-sheet {
      grid-template-columns: auto 1fr;
      align-items: start;
      justify-items: stretch;
      width: min(46rem, 94vw);
      border: 1px solid #d8c6b1;
      border-top: 3px solid #c65f3c;
      box-shadow: 0 20px 60px rgba(27, 20, 16, 0.55);
      animation: peek-in 0.18s ease-out;
    }
    .peek-face {
      width: 17rem;
    }
    .peek-body {
      padding-top: 0.4rem;
      padding-right: 2.2rem;
    }
  }
  @keyframes peek-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  @keyframes peek-up {
    from {
      transform: translateY(100%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .peek-pop,
    .peek-sheet {
      animation: none;
    }
  }
</style>
