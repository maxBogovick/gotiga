<script lang="ts">
  // Расстановка перед волной.
  //
  // Поле лежит вдоль, как в бою: половина игрока слева, врага справа. Сервер
  // называет открытые клетки (`openCells`) и проверяет итог при начале волны;
  // здесь только выбор. Нажать карту (на поле или в ряду под ним), затем клетку:
  // свободная — карта переезжает, занятая другой выбранной картой — они меняются
  // местами. Перетаскивания нет: нажатие — полноценный путь, а не замена
  // (WCAG 2.2, 2.5.7).
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import BattleGroundMark from '$lib/components/BattleGroundMark.svelte';
  import BattleGroundSigil from '$lib/components/BattleGroundSigil.svelte';
  import { GROUND_NAME, GROUND_SHORT, cardCopy } from '$lib/battles';
  import { cellKey, moveTo, threatened, type Placement } from '$lib/runPlacement';
  import type { BattleCard, BattleGround, RunWaveField } from '$lib/types/api';
  import { sfx } from '$lib/sfx';

  let {
    field,
    chosen,
    cells = $bindable(),
    cards,
    onreset,
    onpick,
    onlook,
    compact = false,
  }: {
    field: RunWaveField;
    /** Кто выходит в волну: номера карт отряда. */
    chosen: string[];
    cells: Placement;
    cards: BattleCard[];
    onreset: () => void;
    /** Игрок выбрал свою карту на поле: страница показывает её подробно. */
    onpick?: (cardId: string | null) => void;
    /** Игрок нажал вражескую карту на поле: страница показывает её подробно. */
    onlook?: (slug: string, round?: number) => void;
    /** Экран без прокрутки: поле занимает всю высоту, ряд карт и подсказку даёт страница. */
    compact?: boolean;
  } = $props();

  /** Выбранная карта: следующее нажатие по клетке переставит её. */
  let picked = $state<string | null>(null);
  $effect(() => onpick?.(picked));

  // Зоны ударов включены, пока игрок сам их не выключил: это главная подсказка
  // расстановки, а прятать её за кнопкой значит, что новичок её не увидит.
  const THREATS_KEY = 'gotiga_run_threats';
  function readThreats(): boolean {
    try {
      return localStorage.getItem(THREATS_KEY) !== '0';
    } catch {
      return true;
    }
  }
  let showThreats = $state(readThreats());
  function toggleThreats() {
    showThreats = !showThreats;
    try {
      localStorage.setItem(THREATS_KEY, showThreats ? '1' : '0');
    } catch {
      // Хранилище закрыто — выбор живёт до перезагрузки.
    }
  }

  const rows = $derived(field.depth * 2);
  /** Ряды слева направо: от дальнего края игрока до дальнего края врага. */
  const columns = $derived(Array.from({ length: rows }, (_, i) => rows - 1 - i));
  const lines = $derived(Array.from({ length: field.width }, (_, x) => x));
  const playerHalf = (y: number) => y >= field.depth;

  const terrain = $derived(new Map<string, BattleGround>(field.terrain.map((t) => [cellKey(t.cell), t.ground])));
  const enemyAt = $derived(new Map(field.enemies.map((e) => [cellKey(e), e.slug])));
  const structureAt = $derived(new Map(field.structures.map((e) => [cellKey(e), e.slug])));
  const entranceAt = $derived(
    new Map(field.entrances.map((e) => [cellKey(e), e])),
  );
  const openKeys = $derived(new Set(field.openCells.map(cellKey)));
  const ownAt = $derived(new Map(Object.entries(cells).map(([id, c]) => [cellKey(c), id])));
  const danger = $derived(threatened(field.threats));
  /** Какие земли есть на этом поле: у каждой в подписи значок и имя. */
  const grounds = $derived([...new Set(field.terrain.map((x) => x.ground))]);

  const bySlug = $derived(new Map(cards.map((c) => [c.slug, c])));
  const byId = $derived(new Map(cards.map((c) => [c.id, c])));
  const art = (c: BattleCard | undefined) => c?.artUrlOverride || c?.artUrl || null;
  const slugName = (slug: string) => {
    const c = bySlug.get(slug);
    return c ? cardCopy(c, $lang).title : slug;
  };
  const idName = (id: string) => {
    const c = byId.get(id);
    return c ? cardCopy(c, $lang).title : '';
  };

  /** Нажатие по клетке. Возвращает `true`, если карта переехала или поменялась местами. */
  function press(x: number, y: number): boolean {
    const key = cellKey({ x, y });
    const mine = ownAt.get(key);
    const foe = enemyAt.get(key);
    if (foe && !mine) {
      onlook?.(foe, entranceAt.get(key)?.round);
      return false;
    }
    if (mine) {
      if (picked && picked !== mine) {
        cells = moveTo(cells, picked, { x, y }, field.openCells);
        picked = null;
        sfx.drop();
        return true;
      }
      picked = picked === mine ? null : mine;
      if (picked) sfx.lift();
      return false;
    }
    if (picked && openKeys.has(key)) {
      cells = moveTo(cells, picked, { x, y }, field.openCells);
      picked = null;
      sfx.drop();
      return true;
    }
    return false;
  }

  // Всплывающая карточка клетки: при наведении или фокусе и при нажатии. Нажатая
  // остаётся, пока игрок не нажмёт в другом месте, не нажмёт Esc или не прокрутит страницу.
  type Pop = {
    id: string;
    pinned: boolean;
    left: number;
    bottom?: number;
    top?: number;
    title: string;
    kind?: string;
    tone?: 'mine' | 'foe' | 'ground';
    stats?: string[];
    effect?: string;
    notes: string[];
  };
  const POP_W = 248;
  let pop = $state<Pop | null>(null);

  function place(el: Element): Pick<Pop, 'left' | 'bottom' | 'top'> {
    const r = el.getBoundingClientRect();
    const w = Math.min(POP_W, window.innerWidth - 16);
    const left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, window.innerWidth - w - 8));
    return r.top > 190 ? { left, bottom: window.innerHeight - r.top + 8 } : { left, top: r.bottom + 8 };
  }

  /** Что рассказать про клетку; `null` — рассказывать нечего. */
  function cellInfo(x: number, y: number): Omit<Pop, 'id' | 'pinned' | 'left' | 'bottom' | 'top'> | null {
    const key = cellKey({ x, y });
    const ground = terrain.get(key);
    const mine = ownAt.get(key);
    const foe = enemyAt.get(key);
    const wall = structureAt.get(key);
    const door = entranceAt.get(key);
    const notes: string[] = [];
    let card: BattleCard | undefined;
    let title = '';
    let kind: string | undefined;
    let tone: Pop['tone'];
    if (mine) {
      card = byId.get(mine);
      title = idName(mine);
      kind = $t('runKindMine' as TranslationKey);
      tone = 'mine';
    } else if (foe) {
      card = bySlug.get(foe);
      title = slugName(foe);
      kind = $t('runFoeNow');
      tone = 'foe';
      if (door) notes.push($t('runPlaceEntrance' as TranslationKey).replace('{n}', String(door.round)));
    } else if (wall) {
      card = bySlug.get(wall);
      title = slugName(wall);
      tone = 'foe';
    } else if (ground) {
      title = $t(GROUND_NAME[ground]);
      tone = 'ground';
    }
    if (ground && title !== $t(GROUND_NAME[ground])) notes.push(`${$t(GROUND_NAME[ground])}: ${$t(GROUND_SHORT[ground])}`);
    else if (ground) notes.push($t(GROUND_SHORT[ground]));
    if (danger.has(key) && showThreats) notes.push($t('battleLineupThreatCell'));
    if (!title) {
      if (!(playerHalf(y) && openKeys.has(key))) return null;
      title = $t('battleLineupYourCell');
      tone = 'mine';
      notes.push($t('runPlaceHint'));
    }
    return {
      title,
      kind,
      tone,
      stats: card
        ? [
            `${$t('battlesPowerLabel')} ${card.power}`,
            `${$t('battleStatArmour')} ${card.armor}`,
            `${$t('battleStatWard')} ${card.ward}`,
            `${$t('battleStatReach')} ${card.reach}`,
          ]
        : undefined,
      effect: card ? cardCopy(card, $lang).effect || undefined : undefined,
      notes,
    };
  }

  function showCell(e: Event, x: number, y: number, pinned = false) {
    if (pop?.pinned && !pinned) return;
    const info = cellInfo(x, y);
    if (!info) {
      if (!pop?.pinned) pop = null;
      return;
    }
    pop = { id: cellKey({ x, y }), pinned, ...place(e.currentTarget as Element), ...info };
  }
  function hideHover() {
    if (pop && !pop.pinned) pop = null;
  }
  function clickCell(e: Event, x: number, y: number) {
    const key = cellKey({ x, y });
    const moved = press(x, y);
    const same = pop?.pinned && pop.id === key;
    pop = null;
    if (!moved && !same) showCell(e, x, y, true);
  }
  function showText(e: Event, title: string, text: string) {
    if (pop?.pinned) return;
    pop = { id: `text:${title}`, pinned: false, ...place(e.currentTarget as Element), title, notes: [text] };
  }
  function outside(e: PointerEvent) {
    if (pop?.pinned && !(e.target as Element | null)?.closest?.('.tile')) pop = null;
  }
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  /** Подпись клетки для чтения с экрана: что на ней и можно ли встать. */
  function label(x: number, y: number): string {
    const key = cellKey({ x, y });
    const parts: string[] = [];
    const ground = terrain.get(key);
    if (ground) parts.push($t(GROUND_NAME[ground]));
    const mine = ownAt.get(key);
    if (mine) parts.push(idName(mine));
    const foe = enemyAt.get(key);
    if (foe) parts.push(slugName(foe));
    const wall = structureAt.get(key);
    if (wall) parts.push(slugName(wall));
    const door = entranceAt.get(key);
    if (door) parts.push(`${slugName(door.slug)}. ${$t('runPlaceEntrance' as TranslationKey).replace('{n}', String(door.round))}`);
    if (danger.has(key) && showThreats) parts.push($t('battleLineupThreatCell'));
    if (playerHalf(y) && openKeys.has(key) && !mine) parts.push($t('battleLineupYourCell'));
    return parts.join(' · ');
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={(e) => e.key === 'Escape' && (pop = null)} onscroll={() => (pop = null)} onresize={() => (pop = null)} />

<div class="place" class:place--compact={compact} class:place--carrying={!!picked}>
  <div class="bar">
    <button type="button" class="chip-btn" aria-pressed={showThreats} onclick={toggleThreats}>
      {$t('battleLineupThreats')}
    </button>
    <button
      type="button"
      class="chip-btn"
      aria-label="{$t('runPlaceReset')}. {$t('runPlaceResetAbout')}"
      onmouseenter={(e) => showText(e, $t('runPlaceReset'), $t('runPlaceResetAbout'))}
      onmouseleave={hideHover}
      onfocus={(e) => showText(e, $t('runPlaceReset'), $t('runPlaceResetAbout'))}
      onblur={hideHover}
      onclick={() => ((picked = null), (pop = null), onreset())}
    >{$t('runPlaceReset')}</button>
    {#if compact}<p class="hint hint--bar">{$t('runPlaceHint')}</p>{/if}
  </div>

  <div class="wrap">
    <div class="board" style:--cols={rows} style:--lines={field.width} role="grid" aria-label={$t('runPlace')}>
      <div class="sides" aria-hidden="true">
        <span class="side">{$t('battleLineupYourCell')}</span>
        <span class="side side--foe">{$t('runFoeNow')}</span>
      </div>
      {#each lines as x (x)}
        <div class="row" role="row">
          {#each columns as y (y)}
            {@const key = cellKey({ x, y })}
            {@const ground = terrain.get(key)}
            {@const mine = ownAt.get(key)}
            {@const foe = enemyAt.get(key)}
            {@const wall = structureAt.get(key)}
            {@const door = entranceAt.get(key)}
            {@const usable = !!mine || (playerHalf(y) && openKeys.has(key))}
            <button
              type="button"
              role="gridcell"
              class="tile"
              class:tile--player={playerHalf(y)}
              class:tile--seam={y === field.depth}
              class:tile--open={playerHalf(y) && openKeys.has(key)}
              class:tile--mine={!!mine}
              class:tile--foe={!!foe && !mine}
              class:tile--later={!!door && !!foe && !mine}
              class:tile--picked={!!mine && mine === picked}
              class:tile--threat={showThreats && danger.has(key)}
              class:tile--idle={!usable && !(foe && onlook)}
              aria-disabled={!usable && !(foe && onlook)}
              aria-label={label(x, y)}
              onmouseenter={(e) => showCell(e, x, y)}
              onmouseleave={hideHover}
              onfocus={(e) => showCell(e, x, y)}
              onblur={hideHover}
              onclick={(e) => clickCell(e, x, y)}
            >
              {#if ground}<BattleGroundMark {ground} occupied={!!(mine || foe || wall)} />{/if}
              {#if mine}
                {@const c = byId.get(mine)}
                {#if art(c)}<img class="art" src={art(c)} alt="" draggable="false" />{/if}
                <span class="nm nm--mine">{idName(mine)}</span>
              {:else if foe}
                {@const c = bySlug.get(foe)}
                {#if art(c)}<img class="art art--foe" src={art(c)} alt="" draggable="false" />{/if}
                <span class="nm nm--foe">{slugName(foe)}</span>
              {:else if wall}
                {@const c = bySlug.get(wall)}
                {#if art(c)}<img class="art art--wall" src={art(c)} alt="" draggable="false" />{/if}
              {/if}
              {#if door}<span class="door" aria-hidden="true">{$t('runEntranceBadge')}</span>{/if}
            </button>
          {/each}
        </div>
      {/each}
    </div>
  </div>

  {#if showThreats || field.entrances.length || grounds.length}
    <ul class="legend-row" aria-label={$t('runPlace')}>
      {#if showThreats}
        <li class="legend"><span class="swatch" aria-hidden="true"></span>{$t('battleLineupThreatCell')}</li>
      {/if}
      {#if field.entrances.length}
        <li class="legend"><span class="door door--key" aria-hidden="true">{$t('runEntranceBadge')}</span>{$t('runLegendEntrance')}</li>
      {/if}
      {#each grounds as g (g)}
        <li class="legend">
          <span class="sigil" aria-hidden="true"><BattleGroundSigil ground={g} size="100%" /></span>{$t(GROUND_NAME[g])}: {$t(GROUND_SHORT[g])}
        </li>
      {/each}
    </ul>
  {/if}

  {#if !compact}
  <ul class="tray" aria-label={$t('runPlace')}>
    {#each chosen as id (id)}
      <li>
        <button type="button" class="cardchip" class:on={picked === id} aria-pressed={picked === id} onclick={() => (picked = picked === id ? null : id)}>
          {#if art(byId.get(id))}<img src={art(byId.get(id))} alt="" draggable="false" />{/if}
          <span>{idName(id)}</span>
          {#if !cells[id]}<span class="nocell">{$t('runPlaceNoCell')}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
  <p class="hint">{$t('runPlaceHint')}</p>
  {/if}
</div>

{#if pop}
  <div
    use:portal
    class="pop"
    class:pop--mine={pop.tone === 'mine'}
    class:pop--foe={pop.tone === 'foe'}
    class:pop--ground={pop.tone === 'ground'}
    class:pop--pinned={pop.pinned}
    style:left="{pop.left}px"
    style:width="min({POP_W}px, calc(100vw - 16px))"
    style:bottom={pop.bottom !== undefined ? `${pop.bottom}px` : undefined}
    style:top={pop.top !== undefined ? `${pop.top}px` : undefined}
    aria-hidden="true"
  >
    {#if pop.kind}<p class="pop-kind">{pop.kind}</p>{/if}
    <p class="pop-title">{pop.title}</p>
    {#if pop.stats}
      <ul class="pop-stats">
        {#each pop.stats as st (st)}<li>{st}</li>{/each}
      </ul>
    {/if}
    {#if pop.effect}<p class="pop-effect">{pop.effect}</p>{/if}
    {#each pop.notes as n (n)}<p class="pop-note">{n}</p>{/each}
  </div>
{/if}

<style>
  .place {
    margin: 0.75rem 0;
  }
  .place--compact {
    height: 100%;
    margin: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .place--compact .bar {
    margin-bottom: 0.4rem;
  }
  .place--compact .wrap {
    flex: 1;
    min-height: 0;
    container-type: size;
    display: grid;
    place-items: center;
  }
  .place--compact .tile {
    width: min(
      7rem,
      calc((100cqw - (var(--cols) - 1) * 4px - 14px) / var(--cols)),
      calc((100cqh - (var(--lines) - 1) * 4px - 38px) / var(--lines) * 0.75)
    );
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .chip-btn {
    min-height: 36px;
    padding: 0.2rem 0.8rem;
    border: 1px solid #d8c6b1;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.5);
    color: #6f3b24;
    font: inherit;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .chip-btn[aria-pressed='true'] {
    border-color: #8f2f22;
    background: rgba(143, 47, 34, 0.1);
    color: #6f1f16;
  }
  .wrap {
    container-type: inline-size;
    padding: 8px;
    border: 1px solid #6f3b24;
    background: radial-gradient(ellipse at 50% 40%, #3a2a20 0%, #241a14 75%);
    box-shadow: inset 0 0 0 3px #241a14, inset 0 0 0 4px rgba(216, 198, 177, 0.3), inset 0 0 40px rgba(0, 0, 0, 0.5);
  }
  .board {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    gap: 4px;
    justify-content: center;
  }
  .tile {
    position: relative;
    flex: none;
    width: min(5.2rem, calc((100cqw - (var(--cols) - 1) * 4px - 14px) / var(--cols)));
    aspect-ratio: 3 / 4;
    padding: 0;
    border: 1px solid rgba(216, 198, 177, 0.22);
    background: rgba(143, 47, 34, 0.14);
    overflow: hidden;
    color: #f1e6d4;
    cursor: default;
  }
  .tile--player {
    background: rgba(241, 230, 212, 0.07);
  }
  .tile--seam {
    margin-left: 10px;
  }
  .tile--open {
    cursor: pointer;
    border-style: dashed;
    border-color: rgba(241, 230, 212, 0.6);
  }
  .tile--foe {
    cursor: help;
  }
  /* Подкрепление ещё не вышло на поле: карта приглушена и с пунктирной рамкой. */
  .tile--later {
    border-style: dashed;
  }
  .tile--later .art {
    opacity: 0.55;
  }
  .sides {
    display: flex;
    margin-bottom: 2px;
  }
  .side {
    flex: 1;
    font-size: 0.72rem;
    letter-spacing: 0.14em;
    text-align: center;
    text-transform: uppercase;
    color: rgba(241, 230, 212, 0.75);
  }
  .side--foe {
    color: rgba(214, 120, 100, 0.9);
  }
  .tile--open:hover,
  .tile:focus-visible {
    border-color: #c65f3c;
    outline: 1px solid #c65f3c;
  }
  .tile--mine {
    cursor: pointer;
    border: 2px solid #6f3b24;
    background: #1b1410;
    animation: knock 240ms ease-out;
  }
  .tile--mine .art {
    animation: settle 280ms cubic-bezier(0.2, 0.9, 0.3, 1.15);
  }
  /* Пока выбрана карта, свободные клетки мягко мигают: видно, куда её можно поставить. */
  .place--carrying .tile--open:not(.tile--mine) {
    border-color: #c65f3c;
    animation: invite 1.2s ease-in-out infinite;
  }
  @keyframes knock {
    0% {
      transform: scale(1.1);
    }
    60% {
      transform: scale(0.97);
    }
    100% {
      transform: scale(1);
    }
  }
  @keyframes settle {
    from {
      transform: translateY(-14px) scale(1.25);
      opacity: 0.2;
    }
    to {
      transform: none;
      opacity: 1;
    }
  }
  @keyframes invite {
    0%,
    100% {
      background: rgba(241, 230, 212, 0.07);
    }
    50% {
      background: rgba(198, 95, 60, 0.3);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tile--mine,
    .tile--mine .art,
    .place--carrying .tile--open:not(.tile--mine) {
      animation: none;
    }
  }
  .tile--picked {
    border-color: #c65f3c;
    box-shadow: 0 0 0 2px rgba(198, 95, 60, 0.45);
  }
  .tile--threat::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 0 2px rgba(214, 84, 62, 0.9), inset 0 0 14px rgba(214, 84, 62, 0.45);
  }
  .tile--idle {
    cursor: default;
  }
  .pop {
    position: fixed;
    z-index: 60;
    box-sizing: border-box;
    padding: 0.6rem 0.75rem 0.65rem;
    border: 1px solid #d8c6b1;
    border-left: 3px solid #c65f3c;
    background: #fcf7ee;
    color: #34251c;
    box-shadow: 0 10px 28px rgba(52, 37, 28, 0.38), 0 0 0 3px rgba(248, 241, 231, 0.55);
    font-family: Inter, system-ui, sans-serif;
    font-size: 0.82rem;
    line-height: 1.4;
    pointer-events: none;
    animation: pop-in 0.14s ease-out;
  }
  .pop--foe {
    border-left-color: #8f2f22;
  }
  .pop--ground {
    border-left-color: #6f3b24;
  }
  .pop--pinned {
    box-shadow: 0 12px 32px rgba(52, 37, 28, 0.45), 0 0 0 3px rgba(198, 95, 60, 0.35);
  }
  .pop p {
    margin: 0;
  }
  .pop-kind {
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #6f3b24;
  }
  .pop-title {
    margin: 0.1rem 0 0.35rem !important;
    font-family: Fraunces, Georgia, serif;
    font-size: 1.05rem;
    font-weight: 600;
    line-height: 1.2;
  }
  .pop-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.35rem;
    margin: 0 0 0.4rem;
    padding: 0;
    list-style: none;
  }
  .pop-stats li {
    padding: 0.05rem 0.4rem;
    border: 1px solid #d8c6b1;
    background: #f1e6d4;
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .pop-effect {
    margin-bottom: 0.35rem !important;
    color: #5a4535;
  }
  .pop-note {
    padding-top: 0.35rem;
    border-top: 1px dashed #d8c6b1;
  }
  .pop-title + .pop-note {
    border-top: 0;
    padding-top: 0;
  }
  @keyframes pop-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pop {
      animation: none;
    }
  }
  .art {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .art--foe {
    filter: saturate(0.85) hue-rotate(-12deg) brightness(0.9);
  }
  .art--wall {
    filter: grayscale(0.6) brightness(0.8);
  }
  .nm {
    position: absolute;
    inset: auto 0 0 0;
    padding: 2px 3px;
    font-size: 12px;
    line-height: 1.15;
    text-align: center;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .nm--mine {
    background: rgba(111, 59, 36, 0.88);
    color: #f8f1e7;
  }
  .nm--foe {
    background: rgba(111, 31, 22, 0.82);
    color: #f8f1e7;
  }
  .door {
    position: absolute;
    top: 1px;
    left: 1px;
    padding: 1px 7px;
    border-radius: 999px;
    background: #b08a3a;
    color: #fff;
    font-size: 11px;
    line-height: 1.3;
    white-space: nowrap;
    z-index: 2;
  }
  .legend-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 1rem;
    margin: 0.4rem 0 0;
    padding: 0;
    list-style: none;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.9rem;
    color: #6f3b24;
  }
  .door--key {
    position: static;
  }
  .sigil {
    display: inline-block;
    width: 1.1rem;
    height: 1.1rem;
  }
  .swatch {
    width: 1.1rem;
    height: 1.1rem;
    border: 1px solid #8f2f22;
    background: #3a2a20;
    box-shadow: inset 0 0 0 2px rgba(214, 84, 62, 0.9);
  }
  .tray {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin: 0.75rem 0 0;
    padding: 0;
    list-style: none;
  }
  .cardchip {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    min-height: 44px;
    padding: 0.2rem 0.7rem 0.2rem 0.25rem;
    border: 1px solid #d8c6b1;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.55);
    color: #34251c;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .cardchip img {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    object-fit: cover;
  }
  .cardchip.on {
    border-color: #c65f3c;
    background: rgba(198, 95, 60, 0.14);
    box-shadow: 0 0 0 2px rgba(198, 95, 60, 0.3);
  }
  .nocell {
    color: #8f2f22;
    font-size: 0.75rem;
  }
  .hint.hint--bar {
    flex: 1 1 14rem;
    align-self: center;
    margin: 0;
  }
  .hint {
    margin: 0.5rem 0 0;
    font-size: 0.82rem;
    line-height: 1.45;
    color: #6f3b24;
  }
</style>
