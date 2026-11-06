<script lang="ts">
  // Подкрепление волны на схеме поля.
  //
  // Поле тут то же, что у боя-основы волны, и повёрнуто как на столе поля:
  // половина игрока слева, половина врага справа. Клетка входа выбирается
  // нажатием, а не числами: нажатие на пустую клетку врага ставит туда новую
  // строку или переносит выбранную, нажатие на занятую выбирает её строку.
  import { t, lang, localized } from '$lib/i18n';
  import BattleGroundMark from '$lib/components/BattleGroundMark.svelte';
  import { GROUND_NAME, UNSTANDABLE } from '$lib/battles';
  import type { AdminWave, BattleCard, BattleGround, BattleTile, ChallengeSetup } from '$lib/types/api';

  let {
    setup,
    entries = $bindable(),
    cards,
  } = $props<{
    setup: ChallengeSetup | undefined;
    entries: AdminWave['reinforcements'];
    cards: BattleCard[];
  }>();

  /** Какая строка выбрана: нажатие на пустую клетку переносит её. */
  let selected = $state<number | null>(null);

  const field = $derived(setup?.field ?? { width: 3, depth: 3 });
  const rows = $derived(field.depth * 2);
  /** Ряды слева направо: от дальнего края игрока до дальнего края врага. */
  const columns = $derived(Array.from({ length: rows }, (_, i) => rows - 1 - i));
  const lines = $derived(Array.from({ length: field.width }, (_, x) => x));

  const terrain = $derived(new Map<string, BattleGround>(
      (setup?.terrain ?? []).map((tile: BattleTile) => [`${tile.cell.x},${tile.cell.y}`, tile.ground]),
    ));
  const bodies = $derived(
    new Map<string, string>(
      [...(setup?.playerBoard ?? []), ...(setup?.keeperBoard ?? [])].map((p) => [`${p.x},${p.y}`, p.card]),
    ),
  );
  const bySlug = $derived(new Map<string, BattleCard>(cards.map((c: BattleCard) => [c.slug, c])));
  const nameOf = (slug: string) => {
    const c = bySlug.get(slug);
    return c ? localized(c, 'title', $lang) : slug;
  };
  const isStructure = (slug: string) => bySlug.get(slug)?.kind === 'structure';

  const entryAt = (x: number, y: number) => entries.findIndex((e: AdminWave['reinforcements'][number]) => e.x === x && e.y === y);
  const enemyHalf = (y: number) => y < field.depth;
  /** Куда нельзя ставить: стена, обрыв, чужое строение. */
  const blocked = (x: number, y: number) => {
    const ground = terrain.get(`${x},${y}`);
    const body = bodies.get(`${x},${y}`);
    return (!!ground && UNSTANDABLE.has(ground)) || (!!body && isStructure(body));
  };

  const nextRound = () => entries.reduce((n: number, e: AdminWave['reinforcements'][number]) => Math.max(n, e.round), 0) + 1;
  const lastCard = () => entries[entries.length - 1]?.card ?? cards[0]?.slug ?? '';

  function press(x: number, y: number) {
    if (!enemyHalf(y) || blocked(x, y)) return;
    const at = entryAt(x, y);
    if (at >= 0) {
      selected = selected === at ? null : at;
      return;
    }
    if (selected !== null && entries[selected]) {
      entries[selected].x = x;
      entries[selected].y = y;
      return;
    }
    entries.push({ round: nextRound(), x, y, card: lastCard() });
    selected = entries.length - 1;
  }

  /** Кнопка «добавить»: первая свободная клетка половины врага. */
  function add() {
    for (const y of [...columns].reverse().filter(enemyHalf)) {
      for (const x of lines) {
        if (entryAt(x, y) < 0 && !blocked(x, y)) {
          entries.push({ round: nextRound(), x, y, card: lastCard() });
          selected = entries.length - 1;
          return;
        }
      }
    }
  }

  function drop(i: number) {
    entries.splice(i, 1);
    selected = null;
  }
</script>

<div class="space-y-3">
  <div class="field" style:--cols={rows} role="grid" aria-label={$t('adminRouteColReinf')}>
    {#each lines as x (x)}
      <div class="row" role="row">
        {#each columns as y (y)}
          {@const ground = terrain.get(`${x},${y}`)}
          {@const body = bodies.get(`${x},${y}`)}
          {@const at = entryAt(x, y)}
          {@const live = enemyHalf(y) && !blocked(x, y)}
          <button
            type="button"
            role="gridcell"
            class="tile"
            class:tile--enemy={enemyHalf(y)}
            class:tile--seam={y === field.depth - 1}
            class:tile--live={live}
            class:tile--picked={at >= 0 && at === selected}
            disabled={!live}
            aria-label={`${ground ? `${$t(GROUND_NAME[ground])} · ` : ''}${body ? `${nameOf(body)} · ` : ''}${
              at >= 0 ? `${$t('adminRouteColRound')} ${entries[at].round} · ${nameOf(entries[at].card)}` : ''
            }`}
            onclick={() => press(x, y)}
          >
            {#if ground}<BattleGroundMark {ground} occupied={!!body} />{/if}
            {#if body}<span class="body">{nameOf(body)}</span>{/if}
            {#if at >= 0}
              <span class="entry">
                <b>{entries[at].round}</b>
                <span>{nameOf(entries[at].card)}</span>
              </span>
            {/if}
          </button>
        {/each}
      </div>
    {/each}
  </div>

  {#each entries as e, i (i)}
    <div class="flex gap-2 items-center {selected === i ? 'sel' : ''}">
      <button
        class="w-6 h-6 text-[11px] border border-[#34251c]/30 {selected === i ? 'bg-[#c65f3c] text-white border-[#c65f3c]' : ''}"
        onclick={() => (selected = selected === i ? null : i)}
        aria-label={$t('adminRouteColCard')}>{i + 1}</button
      >
      <label class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] flex items-center gap-1">
        {$t('adminRouteColRound')}
        <input
          class="border border-[#34251c]/20 bg-transparent px-1.5 py-1 text-sm w-16 normal-case tracking-normal text-[#34251c]"
          type="number"
          min="1"
          max="30"
          bind:value={e.round}
        />
      </label>
      <input
        class="border border-[#34251c]/20 bg-transparent px-1.5 py-1 text-sm flex-1 min-w-0"
        bind:value={e.card}
        list="enemy-slugs"
        aria-label={$t('adminRouteColCard')}
      />
      <button
        class="w-8 py-1 text-[10px] border border-[#34251c]/20 hover:bg-[#34251c]/5"
        onclick={() => drop(i)}
        aria-label="remove">×</button
      >
    </div>
  {/each}

  <button
    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
    onclick={add}>+ {$t('adminRouteScheduleAdd')}</button
  >
  <p class="text-[11px] text-[#8a6a55]">{$t('adminRouteScheduleHint')}</p>
</div>

<style>
  .field {
    display: inline-flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    border: 1px solid rgba(52, 37, 28, 0.15);
    max-width: 100%;
    overflow-x: auto;
  }
  .row {
    display: flex;
    gap: 4px;
  }
  .tile {
    position: relative;
    flex: none;
    width: 3.1rem;
    height: 3.1rem;
    padding: 0;
    border: 1px solid rgba(52, 37, 28, 0.18);
    background: repeating-linear-gradient(135deg, rgba(52, 37, 28, 0.05) 0 6px, transparent 6px 12px);
    font-size: 9px;
    line-height: 1.1;
    color: #34251c;
    overflow: hidden;
    cursor: not-allowed;
  }
  .tile--enemy {
    background: rgba(106, 48, 40, 0.05);
  }
  .tile--seam {
    margin-left: 10px;
  }
  .tile--live {
    cursor: pointer;
  }
  .tile--live:hover {
    border-color: rgba(52, 37, 28, 0.55);
  }
  .tile--picked {
    border-color: #c65f3c;
    box-shadow: 0 0 0 2px rgba(198, 95, 60, 0.35);
  }
  .tile:focus-visible {
    outline: 1px solid #c65f3c;
    outline-offset: 1px;
  }
  .body {
    position: absolute;
    inset: auto 0 0 0;
    padding: 1px 2px;
    background: rgba(248, 241, 231, 0.8);
    color: #6f3b24;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    background: rgba(198, 95, 60, 0.16);
    padding: 1px;
  }
  .entry b {
    font-size: 13px;
    color: #8f2f22;
  }
  .entry span {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
