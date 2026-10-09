<script lang="ts">
  /**
   * Стол поля: этюд целиком на одной доске — карты, местность, величина поля,
   * кто ставит половину гостя, название и сохранение.
   *
   * Прежде здесь правилась только земля, а карты ставились на вкладке «Стол»,
   * и кнопка «Сохранить» требовала того, чего на этом экране сделать было
   * нельзя: этюд не сохранялся, а почему — читалось одной красной строкой.
   * Теперь всё, без чего этюд не сохранить, делается здесь же, а то, чего
   * не хватает, названо списком, у каждой строки которого есть кнопка.
   *
   * Работает с ТЕМ ЖЕ этюдом, что и стол хранителя (`benchBoard`,
   * `benchTerrain`, `etudeSide` — общие): этюд один, и два его редактора
   * обязаны видеть одно и то же. Руки сторон и правила остаются на «Столе».
   */
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import BattleGroundMark from '$lib/components/BattleGroundMark.svelte';
  import {
    FIELD_DEPTHS,
    FIELD_WIDTHS,
    GROUNDS,
    GROUND_KEY,
    GROUND_NAME,
    TERRAIN_MAX,
    UNSTANDABLE,
  } from '$lib/battles';
  import type {
    BattleCard,
    BattleChallenge,
    BattleField,
    BattleGround,
    BattlePlayerSide,
  } from '$lib/types/api';

  let {
    challenges,
    etudeId,
    titleRu = $bindable(),
    titleEn = $bindable(),
    board = $bindable(),
    terrain = $bindable(),
    side = $bindable(),
    field,
    cards,
    saving,
    titleOf,
    onopen,
    onsave,
    onresize,
  }: {
    challenges: BattleChallenge[];
    etudeId: string | null;
    /** Название этюда на двух языках — то же, что на столе: этюд один. */
    titleRu: string;
    titleEn: string;
    /** Тела расстановки по клетке, слагами. */
    board: Record<string, string>;
    terrain: Record<string, BattleGround>;
    /** Кто ставит половину гостя: этюд (`scripted`) или колода гостя (`deck`). */
    side: BattlePlayerSide;
    /** Величина поля этюда: клеток поперёк и вглубь у половины. */
    field: BattleField;
    /** Карты, которые можно поставить (на полке и с телом). */
    cards: BattleCard[];
    saving: boolean;
    titleOf: (c: BattleChallenge) => string;
    onopen: (c: BattleChallenge | null) => void;
    onsave: () => void;
    /** Сменить величину. Перекладывает и тела, и землю у шва. */
    onresize: (next: BattleField) => void;
  } = $props();

  // ── Доска ─────────────────────────────────────────────────────────────────

  let rows = $derived(field.depth * 2);
  /** Столбцы на экране: половина гостя слева, хранителя справа — тот же
   *  разворот, что у поля в бою (`spots` в `BattleScene`). */
  let columns = $derived(Array.from({ length: rows }, (_, i) => rows - 1 - i));
  let lines = $derived(Array.from({ length: field.width }, (_, x) => x));
  // Ширину клетки считает CSS по ширине места, отведённого доске (`--cols`):
  // пока она была числом в пикселях, глубокое поле не помещалось рядом с
  // палитрой и уезжало под неё.
  const guestHalf = (y: number) => y >= field.depth;

  // ── Карты ─────────────────────────────────────────────────────────────────

  const nameOf = (c: BattleCard) => ($lang === 'ru' ? c.titleRu || c.titleEn : c.titleEn || c.titleRu);
  let bySlug = $derived(new Map(cards.map((c) => [c.slug, c])));
  const cardTitle = (slug: string) => {
    const c = bySlug.get(slug);
    return c ? nameOf(c) : slug;
  };
  const cardArt = (slug: string) => {
    const c = bySlug.get(slug);
    return c?.artUrlOverride || c?.artUrl || null;
  };

  let search = $state('');
  let shown = $derived(
    cards.filter((c) => {
      const q = search.trim().toLowerCase();
      return !q || c.titleRu.toLowerCase().includes(q) || c.titleEn.toLowerCase().includes(q);
    }),
  );

  // ── Что делаем ────────────────────────────────────────────────────────────

  /** Карты или местность. Начинаем с карт, если их на поле ещё нет: без них
   *  этюд не сохранить, и это первое, что нужно сделать. */
  let mode = $state<'bodies' | 'ground'>(Object.keys(board).length ? 'ground' : 'bodies');
  /** Выбранная карта; `null` — «убрать карту». */
  let pick = $state<string | null>(null);
  /** Кисть земли; `null` — ластик. */
  let brush = $state<BattleGround | null>('wall');
  /** Протягивание: кладёт или стирает — решает первая клетка жеста. */
  let painting = $state<'lay' | 'clear' | null>(null);
  let complaint = $state<string | null>(null);

  let count = $derived(Object.keys(terrain).length);

  // ── Чего не хватает ───────────────────────────────────────────────────────

  let kept = $derived(challenges.find((c) => c.id === etudeId) ?? null);
  let named = $derived(!!titleRu.trim() || !!titleEn.trim());
  let keeperStands = $derived(
    Object.keys(board).some((k) => !guestHalf(Number(k.split(',')[1]))),
  );
  let guestStands = $derived(
    Object.keys(board).some((k) => guestHalf(Number(k.split(',')[1]))),
  );
  let groundUnderBody = $derived(
    Object.entries(terrain).some(([k, g]) => UNSTANDABLE.has(g) && !!board[k]),
  );
  let needs = $derived({
    title: !named,
    keeper: !keeperStands,
    guest: side === 'scripted' && !guestStands,
    ground: groundUnderBody,
  });
  let ready = $derived(!needs.title && !needs.keeper && !needs.guest && !needs.ground);

  /** Отличается ли то, что на столе, от записанного: что угодно — название,
   *  тела, земля, величина, кто ставит половину гостя. Новый этюд — всегда. */
  let changed = $derived.by(() => {
    if (!kept) return true;
    const keyed = (list: { card: string; x: number; y: number }[]) =>
      Object.fromEntries(list.map((p) => [`${p.x},${p.y}`, p.card]));
    const sorted = (o: Record<string, string>) =>
      JSON.stringify(Object.entries(o).sort(([a], [b]) => a.localeCompare(b)));
    const keptBoard = { ...keyed(kept.setup.keeperBoard), ...keyed(kept.setup.playerBoard) };
    const keptGround = Object.fromEntries(
      (kept.setup.terrain ?? []).map((t) => [`${t.cell.x},${t.cell.y}`, t.ground]),
    );
    const keptField = kept.setup.field ?? { width: 3, depth: 3 };
    return (
      titleRu !== kept.titleRu ||
      titleEn !== kept.titleEn ||
      side !== kept.playerSide ||
      field.width !== keptField.width ||
      field.depth !== keptField.depth ||
      sorted(board) !== sorted(keptBoard) ||
      sorted(terrain) !== sorted(keptGround)
    );
  });

  // ── Нажатия на доску ──────────────────────────────────────────────────────

  function placeBody(key: string) {
    const [, y] = key.split(',').map(Number);
    const next = { ...board };
    // Нажатие на ту же карту, что выбрана, или «убрать» — снимает.
    if (pick === null || board[key] === pick) {
      if (!board[key]) return;
      delete next[key];
      board = next;
      return;
    }
    if (terrain[key] && UNSTANDABLE.has(terrain[key])) {
      complaint = `${$t(GROUND_NAME[terrain[key]])}: ${$t('adminBattlesFieldNoStand')}`;
      return;
    }
    if (side === 'deck' && guestHalf(y)) {
      complaint = $t('adminBattlesFieldDeckHalf');
      return;
    }
    next[key] = pick;
    board = next;
  }

  function refusal(key: string, ground: BattleGround): string | null {
    const body = board[key];
    if (body && UNSTANDABLE.has(ground)) {
      return `${cardTitle(body)}: ${$t('adminBattlesFieldUnderBody')} — ${$t(GROUND_NAME[ground])}`;
    }
    if (!terrain[key] && count >= TERRAIN_MAX) return $t('adminBattlesFieldFull');
    return null;
  }

  function lay(key: string, how: 'lay' | 'clear') {
    if (how === 'clear' || brush === null) {
      if (!terrain[key]) return;
      const next = { ...terrain };
      delete next[key];
      terrain = next;
      return;
    }
    if (terrain[key] === brush) return;
    const no = refusal(key, brush);
    if (no) {
      complaint = no;
      return;
    }
    terrain = { ...terrain, [key]: brush };
  }

  function press(key: string, e: PointerEvent) {
    e.preventDefault();
    complaint = null;
    if (mode === 'bodies') {
      placeBody(key);
      return;
    }
    const how = brush === null || terrain[key] === brush ? 'clear' : 'lay';
    painting = how;
    lay(key, how);
  }

  function enter(key: string) {
    if (mode === 'ground' && painting) lay(key, painting);
  }

  /** Отразить землю на другую половину — только в пустые клетки. */
  function mirror() {
    complaint = null;
    const next = { ...terrain };
    let skipped = 0;
    for (const [key, ground] of Object.entries(terrain)) {
      const [x, y] = key.split(',').map(Number);
      const twin = `${x},${rows - 1 - y}`;
      if (terrain[twin]) continue;
      const under = board[twin] && UNSTANDABLE.has(ground);
      const full = !next[twin] && Object.keys(next).length >= TERRAIN_MAX;
      if (under || full) {
        skipped += 1;
        continue;
      }
      next[twin] = ground;
    }
    terrain = next;
    if (skipped) complaint = `${$t('adminBattlesFieldMirrorSkipped')}: ${skipped}`;
  }

  /** Убрать землю из-под всех тел, где на ней не стоят. */
  function clearUnderBodies() {
    const next = { ...terrain };
    for (const [k, g] of Object.entries(terrain)) {
      if (UNSTANDABLE.has(g) && board[k]) delete next[k];
    }
    terrain = next;
  }

  function chooseSide(next: BattlePlayerSide) {
    side = next;
    complaint = null;
  }

  // ── Выбор этюда ───────────────────────────────────────────────────────────

  let menuOpen = $state(false);
  let menuEl = $state<HTMLElement | null>(null);
  let current = $derived(kept ? titleOf(kept) : $t('adminBattlesEtudeNew'));

  function choose(next: BattleChallenge | null) {
    menuOpen = false;
    if ((next?.id ?? null) === etudeId) return;
    // Переключение стирает стол: не сохранённое — спросить, а не потерять.
    const dirty = kept ? changed : Object.keys(board).length > 0 || named || count > 0;
    if (dirty && !confirm($t('adminBattlesFieldDiscard'))) return;
    onopen(next);
  }
</script>

<svelte:window
  onpointerup={() => (painting = null)}
  onpointerdown={(e) => {
    if (menuOpen && menuEl && !menuEl.contains(e.target as Node)) menuOpen = false;
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') menuOpen = false;
  }}
/>

<div class="flex-1 overflow-y-auto min-w-0 px-5 py-4">
  <!-- 1. Этюд, название, сохранение — одна строка. -->
  <div class="flex flex-wrap items-end gap-x-4 gap-y-2">
    <div class="menu" bind:this={menuEl}>
      <span class="label">{$t('adminBattlesEtude')}</span>
      <button
        type="button"
        class="menu-trigger"
        aria-haspopup="listbox"
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <span class="truncate">{current}</span>
        <span class="chevron" aria-hidden="true">▾</span>
      </button>
      {#if menuOpen}
        <ul class="menu-list" role="listbox">
          <li>
            <button
              type="button"
              role="option"
              aria-selected={etudeId === null}
              class="menu-item menu-item--new {etudeId === null ? 'menu-item--on' : ''}"
              onclick={() => choose(null)}>+ {$t('adminBattlesEtudeNew')}</button
            >
          </li>
          {#each challenges as challenge (challenge.id)}
            <li>
              <button
                type="button"
                role="option"
                aria-selected={etudeId === challenge.id}
                class="menu-item {etudeId === challenge.id ? 'menu-item--on' : ''}"
                onclick={() => choose(challenge)}>{titleOf(challenge)}</button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <label class="flex flex-col gap-1">
      <span class="label">{$t('adminBattlesEtudeTitle')} · RU</span>
      <input bind:value={titleRu} placeholder={$t('adminBattlesFieldUntitled')} class="title-input" />
    </label>
    <label class="flex flex-col gap-1">
      <span class="label">{$t('adminBattlesEtudeTitle')} · EN</span>
      <input bind:value={titleEn} class="title-input" />
    </label>
    <span class="ml-auto flex items-center gap-3">
      {#if kept && changed}
        <span class="text-[11px] text-[#c65f3c]">{$t('adminBattlesFieldUnsaved')}</span>
      {/if}
      <button
        onclick={onsave}
        disabled={saving || !ready || !changed}
        class="px-5 py-2 text-[11px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] hover:bg-[#6f3b24] disabled:opacity-35 disabled:cursor-not-allowed"
        >{$t('adminBattlesFieldSave')}</button
      >
    </span>
  </div>

  <!-- 2. Чего не хватает — списком, и у каждой строки есть действие. -->
  {#if !ready}
    <div class="needs" role="status">
      <p class="needs-head">{$t('adminBattlesFieldNeed')}</p>
      <ul>
        {#if needs.title}
          <li><span class="dot"></span>{$t('adminBattlesFieldNeedTitle')}</li>
        {/if}
        {#if needs.keeper}
          <li>
            <span class="dot"></span>{$t('adminBattlesFieldNeedKeeper')}
            <button class="act" onclick={() => (mode = 'bodies')}>{$t('adminBattlesFieldPlaceCard')}</button>
          </li>
        {/if}
        {#if needs.guest}
          <li>
            <span class="dot"></span>{$t('adminBattlesFieldNeedGuest')}
            <button class="act" onclick={() => (mode = 'bodies')}>{$t('adminBattlesFieldPlaceCard')}</button>
            <button class="act" onclick={() => chooseSide('deck')}>{$t('adminBattlesFieldOrDeck')}</button>
          </li>
        {/if}
        {#if needs.ground}
          <li>
            <span class="dot"></span>{$t('adminBattlesFieldNeedNoWall')}
            <button class="act" onclick={clearUnderBodies}>{$t('adminBattlesFieldClearUnder')}</button>
          </li>
        {/if}
      </ul>
    </div>
  {/if}

  <!-- 3. Поле: величина и кто ставит половину гостя. -->
  <div class="flex flex-wrap items-center gap-x-8 gap-y-2 mt-4">
    <span class="flex items-center gap-2">
      <span class="label">{$t('adminBattlesFieldWidth')}</span>
      {#each FIELD_WIDTHS as w (w)}
        <button
          aria-pressed={field.width === w}
          onclick={() => onresize({ ...field, width: w })}
          class="seg {field.width === w ? 'seg--on' : ''}">{w}</button
        >
      {/each}
    </span>
    <span class="flex items-center gap-2">
      <span class="label">{$t('adminBattlesFieldDepth')}</span>
      {#each FIELD_DEPTHS as d (d)}
        <button
          aria-pressed={field.depth === d}
          onclick={() => onresize({ ...field, depth: d })}
          class="seg {field.depth === d ? 'seg--on' : ''}">{d}</button
        >
      {/each}
      <span class="text-[11px] tabular-nums text-[#8a6a55]">= {field.width} × {rows}</span>
    </span>
    <span class="flex items-center gap-2">
      <span class="label">{$t('adminBattlesFieldGuestSide')}</span>
      <button aria-pressed={side === 'scripted'} onclick={() => chooseSide('scripted')} class="seg seg--wide {side === 'scripted' ? 'seg--on' : ''}"
        >{$t('adminBattlesFieldGuestScripted')}</button
      >
      <button aria-pressed={side === 'deck'} onclick={() => chooseSide('deck')} class="seg seg--wide {side === 'deck' ? 'seg--on' : ''}"
        >{$t('adminBattlesFieldGuestDeck')}</button
      >
    </span>
  </div>

  <!-- 4. Палитра слева, доска справа: доска занимает всё оставшееся место. -->
  <div class="workbench">
    <div class="palette">
      <div class="modes" role="tablist">
        <button role="tab" aria-selected={mode === 'bodies'} class="mode {mode === 'bodies' ? 'mode--on' : ''}" onclick={() => (mode = 'bodies')}>
          {$t('adminBattlesFieldModeBodies')} <span class="mode-count">{Object.keys(board).length}</span>
        </button>
        <button role="tab" aria-selected={mode === 'ground'} class="mode {mode === 'ground' ? 'mode--on' : ''}" onclick={() => (mode = 'ground')}>
          {$t('adminBattlesFieldModeGround')} <span class="mode-count">{count}/{TERRAIN_MAX}</span>
        </button>
      </div>

      <div class="flex flex-col gap-1">
        {#if mode === 'bodies'}
          <input bind:value={search} placeholder={$t('adminBattlesFieldSearch')} class="search" />
          <button
            onclick={() => (pick = null)}
            class="pick {pick === null ? 'pick--on' : ''}"
          >
            <span class="pick-art pick-art--erase" aria-hidden="true"></span>
            <span class="text-[12px]">{$t('adminBattlesFieldRemoveBody')}</span>
          </button>
          <div class="cards">
            {#each shown as c (c.id)}
              <button onclick={() => (pick = c.slug)} class="pick {pick === c.slug ? 'pick--on' : ''}">
                {#if c.artUrlOverride || c.artUrl}
                  <img class="pick-art" src={c.artUrlOverride || c.artUrl} alt="" loading="lazy" />
                {:else}
                  <span class="pick-art" aria-hidden="true"></span>
                {/if}
                <span class="min-w-0">
                  <span class="block text-[12px] truncate">{nameOf(c)}</span>
                  <span class="block text-[10px] text-[#8a6a55] tabular-nums">
                    {$t('adminBattlesFieldTier')} {c.tier} · ♥ {c.health} · ⚔ {c.power}
                  </span>
                </span>
              </button>
            {/each}
          </div>
          <p class="mt-2 text-[11px] leading-snug text-[#8a6a55]">{$t('adminBattlesFieldPickHint')}</p>
        {:else}
          {#each GROUNDS as g (g)}
            <button onclick={() => (brush = g)} class="pick {brush === g ? 'pick--on' : ''}">
              <span class="swatch"><BattleGroundMark ground={g} /></span>
              <span class="min-w-0">
                <span class="block text-[12px] capitalize">{$t(GROUND_NAME[g])}</span>
                <span class="block text-[10px] leading-snug text-[#8a6a55]">{$t(GROUND_KEY[g])}</span>
              </span>
            </button>
          {/each}
          <button onclick={() => (brush = null)} class="pick {brush === null ? 'pick--on' : ''}">
            <span class="swatch swatch--erase" aria-hidden="true"></span>
            <span class="text-[12px]">{$t('adminBattlesFieldErase')}</span>
          </button>
          <div class="flex flex-wrap gap-2 mt-3">
            <button onclick={mirror} disabled={!count} class="tool">{$t('adminBattlesFieldMirror')}</button>
            <button onclick={() => (terrain = {})} disabled={!count} class="tool">{$t('adminBattlesFieldClear')}</button>
          </div>
          <p class="mt-2 text-[11px] leading-snug text-[#8a6a55]">{$t('adminBattlesFieldHint')}</p>
        {/if}
      </div>
    </div>

    <!-- Доска — вдоль, как в бою. Ширина клетки — от ширины этого места. -->
    <div class="board">
      <p class="mb-2 flex justify-between text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
        <span>← {$t('adminBattlesBenchGuestHalf')}{side === 'deck' ? ` · ${$t('adminBattlesFieldGuestByDeck')}` : ''}</span>
        <span>{$t('adminBattlesBenchKeeperHalf')} →</span>
      </p>
      <div class="field" class:field--ground={mode === 'ground'} role="grid" aria-label={$t('adminBattlesFieldView')} style="--cols:{rows}">
        {#each lines as x (x)}
          <div class="row" role="row">
            {#each columns as y (y)}
              {@const key = `${x},${y}`}
              {@const ground = terrain[key]}
              {@const body = board[key]}
              {@const art = body ? cardArt(body) : null}
              <button
                type="button"
                role="gridcell"
                class="tile"
                class:tile--keeper={y < field.depth}
                class:tile--seam={y === field.depth - 1}
                class:tile--deck={side === 'deck' && guestHalf(y)}
                aria-label={`${key}${ground ? ` — ${$t(GROUND_NAME[ground])}` : ''}${body ? ` — ${cardTitle(body)}` : ''}`}
                onpointerdown={(e) => press(key, e)}
                onpointerenter={() => enter(key)}
              >
                {#if ground}
                  <BattleGroundMark {ground} occupied={!!body} />
                {/if}
                {#if body}
                  {#if art}<img class="body-art" src={art} alt="" draggable="false" />{/if}
                  <span class="body">{cardTitle(body)}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/each}
      </div>
      {#if complaint}
        <p class="mt-2 max-w-[32rem] text-[12px] text-[#8f2f22]" role="status">{complaint}</p>
      {/if}
      <p class="mt-3 text-[11px] text-[#8a6a55]">{$t('adminBattlesFieldHands')}</p>
    </div>
  </div>
</div>

<style>
  .label {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .title-input {
    width: 16rem;
    padding: 0.4rem 0.6rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    background: #fffdf8;
    border: 1px solid rgba(52, 37, 28, 0.25);
    outline: none;
  }

  .title-input:focus {
    border-color: #6f3b24;
  }

  /* Чего не хватает: светлая плашка с акцентной кромкой, а не красный абзац. */
  .needs {
    margin-top: 0.9rem;
    padding: 0.6rem 0.9rem;
    border-left: 3px solid #c65f3c;
    background: rgba(198, 95, 60, 0.07);
  }

  .needs-head {
    margin: 0 0 0.35rem;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #8f2f22;
  }

  .needs ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 13px;
    color: #34251c;
  }

  .needs li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #c65f3c;
  }

  .act {
    padding: 0.15rem 0.6rem;
    font-size: 11px;
    border: 1px solid rgba(111, 59, 36, 0.5);
    color: #6f3b24;
    background: #fffdf8;
  }

  .act:hover {
    background: #6f3b24;
    color: #f8f1e7;
  }

  .seg {
    min-width: 2rem;
    padding: 0.3rem 0.5rem;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    border: 1px solid rgba(52, 37, 28, 0.2);
  }

  .seg--wide {
    font-size: 12px;
  }

  .seg:hover {
    background: rgba(52, 37, 28, 0.05);
  }

  .seg--on,
  .seg--on:hover {
    background: #34251c;
    color: #f8f1e7;
    border-color: #34251c;
  }

  /* Этюды: выпадающий список вместо колонки слева, которая отнимала у доски
     четверть ширины. */
  .menu {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    width: 14rem;
  }

  .menu-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    text-align: left;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    background: #fffdf8;
    border: 1px solid rgba(52, 37, 28, 0.25);
  }

  .menu-trigger:hover,
  .menu-trigger[aria-expanded='true'] {
    border-color: #6f3b24;
  }

  .chevron {
    flex-shrink: 0;
    font-size: 12px;
    color: #8a6a55;
  }

  .menu-list {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 30;
    min-width: 100%;
    max-width: 24rem;
    max-height: min(24rem, 60vh);
    margin: 2px 0 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
    background: #fffdf8;
    border: 1px solid rgba(52, 37, 28, 0.3);
    box-shadow: 0 8px 24px rgba(52, 37, 28, 0.18);
  }

  .menu-item {
    display: block;
    width: 100%;
    padding: 0.5rem 0.75rem;
    text-align: left;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 16px;
    line-height: 1.25;
    border-bottom: 1px solid rgba(52, 37, 28, 0.06);
  }

  .menu-item:hover {
    background: rgba(52, 37, 28, 0.05);
  }

  .menu-item--on {
    background: rgba(52, 37, 28, 0.08);
  }

  .menu-item--new {
    font-family: inherit;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  /* Палитра фиксированной ширины, доска — всё остальное. На узком окне
     доска встаёт под палитру. */
  .workbench {
    display: grid;
    grid-template-columns: 15rem minmax(0, 1fr);
    gap: 1.5rem;
    align-items: start;
    margin-top: 0.75rem;
  }

  @media (max-width: 900px) {
    .workbench {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .board {
    container-type: inline-size;
    min-width: 0;
  }

  .modes {
    display: flex;
    gap: 0;
    margin: 0 0 0.75rem;
    border-bottom: 1px solid rgba(52, 37, 28, 0.15);
  }

  .mode {
    padding: 0.5rem 0.9rem;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }

  .mode--on {
    color: #34251c;
    border-bottom-color: #34251c;
  }

  .mode-count {
    margin-left: 0.3rem;
    font-size: 10px;
    letter-spacing: 0;
    color: #8a6a55;
  }

  .search {
    width: 100%;
    padding: 0.35rem 0.5rem;
    margin-bottom: 0.3rem;
    font-size: 12px;
    background: #fffdf8;
    border: 1px solid rgba(52, 37, 28, 0.2);
    outline: none;
  }

  .cards {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: min(26rem, 52vh);
    overflow-y: auto;
  }

  .pick {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    text-align: left;
    padding: 0.3rem 0.4rem;
    border: 1px solid transparent;
  }

  .pick:hover {
    background: rgba(52, 37, 28, 0.03);
  }

  .pick--on,
  .pick--on:hover {
    border-color: #34251c;
    background: rgba(52, 37, 28, 0.06);
  }

  .pick-art {
    flex-shrink: 0;
    width: 30px;
    height: 40px;
    object-fit: cover;
    background: #e9dfd0;
    border: 1px solid rgba(52, 37, 28, 0.15);
  }

  .pick-art--erase,
  .swatch--erase {
    background:
      linear-gradient(45deg, transparent 47%, rgba(143, 47, 34, 0.6) 48% 52%, transparent 53%),
      #fbf6ee;
  }

  .swatch {
    position: relative;
    flex-shrink: 0;
    width: 30px;
    height: 40px;
    border: 1px solid rgba(52, 37, 28, 0.15);
    background: #fbf6ee;
  }

  .tool {
    padding: 0.3rem 0.6rem;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    border: 1px solid rgba(52, 37, 28, 0.2);
  }

  .tool:disabled {
    opacity: 0.4;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    user-select: none;
    touch-action: none;
  }

  .row {
    display: flex;
    gap: 4px;
  }

  /* Клетка — доля ширины доски за вычетом промежутков и шва, но не крупнее 104px:
     поле 3 × 6 на широком окне не должно раздуваться в плакат. */
  .tile {
    position: relative;
    width: min(104px, calc((100cqw - (var(--cols) - 1) * 4px - 10px) / var(--cols)));
    aspect-ratio: 3 / 4;
    overflow: visible;
    border: 1px solid rgba(52, 37, 28, 0.18);
    background: rgba(42, 64, 104, 0.04);
    cursor: pointer;
  }

  .field--ground .tile {
    cursor: crosshair;
  }

  .tile--keeper {
    background: rgba(106, 48, 40, 0.05);
  }

  /* Половина гостя, которую принесёт его колода: здесь этюд ничего не ставит. */
  .tile--deck {
    background: repeating-linear-gradient(135deg, rgba(52, 37, 28, 0.04) 0 6px, transparent 6px 12px);
  }

  /* Шов между половинами. */
  .tile--seam {
    margin-left: 10px;
  }

  .tile--seam::before {
    content: '';
    position: absolute;
    top: -3px;
    bottom: -3px;
    left: -8px;
    border-left: 1px dashed rgba(52, 37, 28, 0.3);
  }

  .tile:hover {
    border-color: rgba(52, 37, 28, 0.5);
  }

  .tile:focus-visible {
    outline: 1px solid #c65f3c;
    outline-offset: 1px;
  }

  /* Карта на доске — её картинка во всю клетку и имя внизу: узнают по лицу. */
  .body-art {
    position: absolute;
    inset: 2px;
    z-index: 1;
    width: calc(100% - 4px);
    height: calc(100% - 4px);
    object-fit: cover;
    pointer-events: none;
  }

  .body {
    position: absolute;
    inset: auto 2px 2px 2px;
    z-index: 2;
    padding: 2px 4px;
    font-size: 10px;
    line-height: 1.2;
    text-align: center;
    color: #f8f1e7;
    background: rgba(52, 37, 28, 0.85);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    pointer-events: none;
  }
</style>
