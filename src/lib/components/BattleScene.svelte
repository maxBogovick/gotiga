<script lang="ts">
  // Стол с фигурами, а не панель показателей.
  //
  // Ни одного правила здесь нет и быть не должно: пометки выводятся из
  // `legalActions`, которые уже посчитал движок. Клиент не знает ни дальности,
  // ни маны, ни покрова — он знает, куда можно ткнуть. Всё, что сцена делает
  // сама, — это показывает и выдерживает паузы.
  //
  // Описание комнаты целиком — `BATTLE-SCENE.md`. Всё, что ниже похоже на
  // произвол, объяснено там.
  import { onMount, untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t, type TranslationKey } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import { DEFAULT_ASPECT, frameForCard } from '$lib/battles';
  import type {
    BattleAction,
    BattleCard as BattleCardDto,
    BattleCell,
    BattleEvent,
    BattleFrame,
    BattleMatch,
    BattleMatchState,
    BattleUnit,
  } from '$lib/types/api';

  let {
    match,
    cards = [],
    frames = null,
    busy = false,
    control = 'player',
    onact,
    onreplay,
    onleave,
  }: {
    match: BattleMatch;
    cards?: BattleCardDto[];
    /** Рамки чинов. Без них карта наденет рамку по умолчанию. */
    frames?: BattleFrame[] | null;
    busy?: boolean;
    /** `both` — стол хранителя: ходить можно за обе стороны. */
    control?: 'player' | 'both';
    onact: (action: BattleAction) => void;
    /** Под печатью. Обе необязательны: у стола хранителя их нет. */
    onreplay?: () => void;
    onleave?: () => void;
  } = $props();

  const WIDTH = 3;
  const DEPTH = 6;

  // ── Темп ──────────────────────────────────────────────────────────────────
  // Числа из BATTLE-SCENE.md §6. Держатся здесь, рядом друг с другом, чтобы
  // менять их можно было как один ритм, а не по одному в четырёх местах.
  const BEAT = { played: 300, moved: 350, damaged: 400, healed: 300, died: 500 };
  const REST = 500;
  const LUNGE_OUT = 220;
  const LUNGE_BACK = 180;

  /** `prefers-reduced-motion` — не украшение, а обязательство: при нём весь ход
   *  применяется мгновенно, а разбор остаётся доступен. */
  let calm = $state(false);
  onMount(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    calm = mq.matches;
    const listen = () => (calm = mq.matches);
    mq.addEventListener('change', listen);
    return () => mq.removeEventListener('change', listen);
  });

  // ── Что показано ──────────────────────────────────────────────────────────
  //
  // Обычно — ровно то, что прислал сервер. Во время проигрывания журнала —
  // промежуточная запись: события переписываются на снимок предыдущей позиции
  // по одному. Это не счёт правил, а перепись готового ответа: событие уже
  // говорит, сколько снято и куда шагнули. По окончании доска снимается со
  // снимка сервера, так что любое расхождение живёт две секунды и умирает.
  let live = $state<BattleMatchState | null>(null);
  let position = $derived(live ?? match.state);
  let legal = $derived(match.legalActions);

  /** Снимок, с которого начнётся проигрывание следующей посылки. Не руна:
   *  ничего от него не перерисовывается. */
  let before: BattleMatchState | null = null;
  /** Для доски: пока журнал играет, она не принимает касаний. */
  let playing = $state(false);
  /** То же самое, но не руна: эффект ниже обязан читать это без подписки. */
  let running = false;
  let run = 0;

  let me = $derived(control === 'both' ? position.active : 'player');
  let mine = $derived(position.active === me && !position.outcome && !busy && !playing);

  const titleOf = (slug: string) =>
    cards.find((c) => c.slug === slug)?.titleRu || slug;
  const dtoOf = (slug: string) => cards.find((c) => c.slug === slug) ?? null;

  /** Отношение сторон рамки этой карты. Клетке нужно знать его заранее: карта,
   *  которой позволено мерить себя по содержимому, из клетки вылезает. */
  const aspectOf = (dto: BattleCardDto) =>
    frameForCard(dto, frames).aspect || DEFAULT_ASPECT;

  let byCell = $derived(
    new Map(position.board.map((s) => [`${s.cell.x},${s.cell.y}`, s.unit])),
  );
  const unitAt = (x: number, y: number): BattleUnit | null => {
    const id = byCell.get(`${x},${y}`);
    return id === undefined ? null : (position.units[id] ?? null);
  };
  const cellOf = (unit: number): BattleCell | null => {
    const spot = position.board.find((s) => s.unit === unit);
    return spot ? spot.cell : null;
  };

  // ── Выбор ─────────────────────────────────────────────────────────────────
  type Picked = { kind: 'unit'; id: number } | { kind: 'hand'; index: number } | null;
  let picked = $state<Picked>(null);

  /** Клетки, куда выбранное может встать или шагнуть. */
  let openCells = $derived.by(() => {
    const out = new Set<string>();
    if (!picked || !mine) return out;
    for (const a of legal) {
      if (typeof a === 'string') continue;
      if (picked.kind === 'hand' && 'play' in a && a.play.handIndex === picked.index) {
        out.add(`${a.play.cell.x},${a.play.cell.y}`);
      }
      if (picked.kind === 'unit' && 'move' in a && a.move.unit === picked.id) {
        out.add(`${a.move.to.x},${a.move.to.y}`);
      }
    }
    return out;
  });

  /** Тела, по которым выбранное может ударить или которые может залечить. */
  let openUnits = $derived.by(() => {
    const out = new Map<number, 'attack' | 'mend'>();
    if (picked?.kind !== 'unit' || !mine) return out;
    for (const a of legal) {
      if (typeof a === 'string') continue;
      if ('attack' in a && a.attack.attacker === picked.id) out.set(a.attack.target, 'attack');
      if ('mend' in a && a.mend.healer === picked.id) out.set(a.mend.target, 'mend');
    }
    return out;
  });

  /** Тела, которым вообще есть чем ходить, — чтобы не тыкать в пустое. */
  let ready = $derived.by(() => {
    const out = new Set<number>();
    for (const a of legal) {
      if (typeof a === 'string') continue;
      if ('move' in a) out.add(a.move.unit);
      if ('attack' in a) out.add(a.attack.attacker);
      if ('mend' in a) out.add(a.mend.healer);
    }
    return out;
  });

  let hand = $derived(me === 'player' ? position.player.hand : position.keeper.hand);
  let theirHand = $derived(me === 'player' ? position.keeper.hand : position.player.hand);

  let playableHand = $derived.by(() => {
    const out = new Set<number>();
    for (const a of legal) {
      if (typeof a !== 'string' && 'play' in a) out.add(a.play.handIndex);
    }
    return out;
  });

  /**
   * Почему из руки нельзя выложить ни одной карты.
   *
   * Движок про это молчит: он присылает список законных действий, и если
   * выложить нечего, «выложить» в нём просто нет. Отличить «не хватает маны» от
   * «некуда ставить» по пустому списку невозможно, и человек видел только
   * тускловатые карты, которые не нажимаются, — без единого слова о причине.
   *
   * Сравнение цены с маной здесь — не второе правило игры, а объяснение уже
   * принятого сервером решения: играть по нему нельзя, оно только называет то,
   * что и так видно на карте.
   */
  let handTrouble = $derived.by((): 'mana' | 'room' | null => {
    if (!mine || !hand.length || playableHand.size > 0) return null;
    const cheapest = Math.min(...hand.map((h) => h.cost));
    return cheapest > position.player.mana ? 'mana' : 'room';
  });

  let chosen = $derived(picked?.kind === 'unit' ? (position.units[picked.id] ?? null) : null);

  function tapCell(x: number, y: number) {
    if (!mine) return;
    const here = unitAt(x, y);

    if (here && openUnits.has(here.id)) {
      const how = openUnits.get(here.id);
      const source = picked as { kind: 'unit'; id: number };
      onact(
        how === 'attack'
          ? { attack: { attacker: source.id, target: here.id } }
          : { mend: { healer: source.id, target: here.id } },
      );
      return;
    }

    if (!here && picked && openCells.has(`${x},${y}`)) {
      const cell: BattleCell = { x, y };
      onact(
        picked.kind === 'hand'
          ? { play: { handIndex: picked.index, cell } }
          : { move: { unit: picked.id, to: cell } },
      );
      return;
    }

    // Иначе — просто выбор своего тела, или снятие выбора. Пустое место снимает.
    if (here && here.owner === me && ready.has(here.id)) {
      picked =
        picked?.kind === 'unit' && picked.id === here.id ? null : { kind: 'unit', id: here.id };
    } else {
      picked = null;
    }
  }

  function tapHand(index: number) {
    if (!mine || !playableHand.has(index)) return;
    picked = picked?.kind === 'hand' && picked.index === index ? null : { kind: 'hand', index };
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && picked) {
      picked = null;
      e.stopPropagation();
    }
  }

  // ── Следы ударов ──────────────────────────────────────────────────────────
  //
  // Линия от бьющего к цели. Первые `400 мс` в полную силу, дальше бледнеет —
  // но не исчезает до конца хода: наводить можно только на то, что видно.
  type Trace = {
    key: number;
    from: BattleCell;
    to: BattleCell;
    by: string;
    at: string;
    trail: { step: string; from: number; to: number }[];
    total: number;
    immune: boolean;
    fresh: boolean;
  };
  let traces = $state<Trace[]>([]);
  let reading = $state<number | null>(null);
  let traceKey = 0;

  // ── Движение ──────────────────────────────────────────────────────────────
  // Одно тело подаётся, другое вздрагивает, третье оседает. Пишется свойствами,
  // а не классами, потому что подача — это направление, а его знает только сцена.
  let lunge = $state<{ unit: number; dx: number; dy: number } | null>(null);
  let flinch = $state<number | null>(null);
  let falling = $state<number | null>(null);

  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  /**
   * Снимок позиции, отвязанный от рун.
   *
   * Не `structuredClone`: то, что приходит сюда, — прокси Svelte, и он на них
   * падает. Позиция целиком состоит из чисел, строк и списков, так что круг
   * через JSON — честная копия, а не приближение.
   */
  const copy = <T,>(x: T): T => JSON.parse(JSON.stringify(x)) as T;

  /** Переписать одно событие на показанную позицию. Не решение — перепись. */
  function transcribe(pos: BattleMatchState, e: BattleEvent) {
    if ('played' in e) {
      // Тело, которого в снимке ещё нет: берётся из окончательной позиции,
      // но целым — карта выходит на поле целой, а раны придут своими событиями.
      const born = copy(match.state.units[e.played.unit]);
      if (born) {
        born.health = { ...born.health, current: born.health.max };
        pos.units[e.played.unit] = born;
        pos.board = [...pos.board, { cell: e.played.cell, unit: e.played.unit }];
      }
    } else if ('moved' in e) {
      pos.board = pos.board.map((s) =>
        s.unit === e.moved.unit ? { ...s, cell: e.moved.to } : s,
      );
    } else if ('damaged' in e) {
      const u = pos.units[e.damaged.target];
      if (u) {
        u.shield = Math.max(0, u.shield - e.damaged.toShield);
        u.health = { ...u.health, current: Math.max(0, u.health.current - e.damaged.toHealth) };
      }
    } else if ('healed' in e) {
      const u = pos.units[e.healed.target];
      if (u) {
        u.health = {
          ...u.health,
          current: Math.min(u.health.max, u.health.current + e.healed.amount),
        };
      }
    } else if ('died' in e) {
      pos.board = pos.board.filter((s) => s.unit !== e.died.target);
    }
  }

  /** След от бьющего к цели — если известно, кто бил и где обе стороны стоят. */
  function markTrace(pos: BattleMatchState, by: number | null, target: number, e: BattleEvent) {
    if (by == null) return;
    const spotOf = (id: number) => pos.board.find((s) => s.unit === id)?.cell ?? null;
    const from = spotOf(by);
    const to = spotOf(target);
    if (!from || !to) return;
    const dmg = 'damaged' in e ? e.damaged : null;
    const trace: Trace = {
      key: ++traceKey,
      from,
      to,
      by: titleOf(pos.units[by]?.card.name ?? ''),
      at: titleOf(pos.units[target]?.card.name ?? ''),
      trail: dmg?.trail ?? [],
      total: dmg ? dmg.toHealth + dmg.toShield : 0,
      immune: !dmg,
      fresh: true,
    };
    traces = [...traces, trace];
    if (!calm) {
      const key = trace.key;
      setTimeout(() => {
        traces = traces.map((x) => (x.key === key ? { ...x, fresh: false } : x));
      }, BEAT.damaged);
    } else {
      trace.fresh = false;
    }
  }

  /** Показать одно событие: движение, потом перепись, потом пауза. */
  async function beat(pos: BattleMatchState, e: BattleEvent, token: number) {
    const alive = () => run === token;

    if ('moved' in e) {
      transcribe(pos, e);
      live = copy(pos);
      if (!calm) await sleep(BEAT.moved);
      return alive();
    }

    if ('damaged' in e || 'immune' in e) {
      const target = 'damaged' in e ? e.damaged.target : e.immune.target;
      const by = 'damaged' in e ? e.damaged.by : e.immune.by;
      markTrace(pos, by, target, e);

      if (!calm && by != null) {
        const a = pos.board.find((s) => s.unit === by)?.cell;
        const b = pos.board.find((s) => s.unit === target)?.cell;
        if (a && b) {
          // Треть клетки в сторону цели: удар должен читаться без единого числа,
          // и должно быть видно, кто ударил, — при дальности 4 иначе никак.
          const len = Math.max(1, Math.abs(b.x - a.x) + Math.abs(b.y - a.y));
          lunge = { unit: by, dx: ((b.x - a.x) / len) * 33, dy: ((b.y - a.y) / len) * 33 };
          await sleep(LUNGE_OUT);
          if (!alive()) return false;
        }
      }

      // Урон показывается тем, что меняется здоровье цели. Ничего не взлетает.
      transcribe(pos, e);
      live = copy(pos);
      if ('damaged' in e && !calm) {
        flinch = target;
        setTimeout(() => (flinch = null), 160);
      }
      if (!calm) {
        lunge = null;
        await sleep(LUNGE_BACK);
      }
      return alive();
    }

    if ('died' in e) {
      if (!calm) {
        falling = e.died.target;
        await sleep(BEAT.died);
        falling = null;
        if (!alive()) return false;
      }
      transcribe(pos, e);
      live = copy(pos);
      return alive();
    }

    transcribe(pos, e);
    live = copy(pos);
    if (!calm && ('played' in e || 'healed' in e)) {
      await sleep('played' in e ? BEAT.played : BEAT.healed);
    }
    return alive();
  }

  /**
   * Ход пришёл целиком: и своё действие, и весь ответ хранителя. Граница между
   * ними — `turnEnded` своей стороны. До неё применяется мгновенно (рука должна
   * чувствоваться своей), после — по одному, с паузами.
   */
  async function playThrough(from: BattleMatchState, events: BattleEvent[]) {
    const token = ++run;
    const pos = copy(from);

    let cut = events.findIndex((e) => 'turnEnded' in e && e.turnEnded.side === me);
    cut = cut < 0 ? events.length : cut + 1;

    for (let i = 0; i < cut; i++) transcribe(pos, events[i]);
    const tail = events.slice(cut);

    if (!tail.length || calm) {
      for (const e of tail) {
        const d = 'damaged' in e ? e.damaged : 'immune' in e ? e.immune : null;
        if (d) markTrace(pos, d.by, d.target, e);
        transcribe(pos, e);
      }
      live = null;
      before = match.state;
      return;
    }

    live = pos;
    running = true;
    playing = true;
    picked = null;
    for (const e of tail) {
      // Пауза — между тем, что видно. Конец хода и итог ничего не показывают,
      // и полсекунды тишины перед ними человек читает как задержку, а не как ритм.
      if (!('turnEnded' in e) && !('finished' in e)) {
        await sleep(REST);
        if (run !== token) return;
      }
      if (!(await beat(pos, e, token))) return;
    }
    if (run !== token) return;
    running = false;
    playing = false;
    live = null;
    before = match.state;
  }

  /**
   * Новая посылка пришла.
   *
   * Зависимость одна — журнал: у стола хранителя `seq` всегда ноль, а журнал
   * приходит новым массивом на каждый ответ. Всё остальное читается под
   * `untrack`, иначе эффект подпишется на то, что сам же и пишет, — и первый же
   * `playing = true` перезапустил бы его, оборвав собственное проигрывание.
   */
  $effect(() => {
    const events = match.events;
    untrack(() => arrive(events));
  });

  function arrive(events: BattleEvent[]) {
    picked = null;
    reading = null;
    lunge = null;
    flinch = null;
    falling = null;
    traces = [];

    if (before === null || running) {
      // Первый показ — или посылка догнала предыдущую, пока играла прошлая:
      // снимок сервера и есть истина, играть нечего.
      run++;
      running = false;
      playing = false;
      live = null;
      before = match.state;
      return;
    }

    void playThrough(before, events);
  }

  // ── Журнал ────────────────────────────────────────────────────────────────
  const STEP_KEY: Record<string, TranslationKey> = {
    immunity: 'battleStepImmunity',
    pointBlank: 'battleStepPointBlank',
    attackerBless: 'battleStepAttackerBless',
    attackerCurse: 'battleStepAttackerCurse',
    targetVulnerable: 'battleStepTargetVulnerable',
    channelDefence: 'battleStepChannelDefence',
    floor: 'battleStepFloor',
    shield: 'battleStepShield',
  };

  /** Признак шага — в слово. Незнакомый шаг показывается как есть: движок может
   *  завести новый раньше, чем словарь, и честнее показать признак, чем чужое
   *  слово вместо него. */
  function stepWord(step: string, tr: (key: TranslationKey) => string): string {
    const key = STEP_KEY[step] as TranslationKey | undefined;
    return key ? tr(key) : step;
  }

  const nameOf = (id: number) => titleOf(match.state.units[id]?.card.name ?? String(id));

  type Line = { text: string; trail: Trace['trail']; total: number; head: string };
  let journal = $derived.by<Line[]>(() =>
    match.events.map((e) => {
      const bare = (text: string): Line => ({ text, trail: [], total: 0, head: '' });
      if ('played' in e) return bare(`${nameOf(e.played.unit)} — ${$t('battleLogPlayed')}`);
      if ('moved' in e) return bare(`${nameOf(e.moved.unit)} — ${$t('battleLogMoved')}`);
      if ('healed' in e)
        return bare(`${nameOf(e.healed.target)} — ${$t('battleLogHealed')} ${e.healed.amount}`);
      if ('died' in e) return bare(`${nameOf(e.died.target)} — ${$t('battleLogDied')}`);
      if ('immune' in e) return bare(`${nameOf(e.immune.target)} — ${$t('battleLogImmune')}`);
      if ('turnEnded' in e)
        return bare(
          e.turnEnded.side === me ? $t('battleLogTurnYours') : $t('battleLogTurnKeeper'),
        );
      if ('damaged' in e) {
        const d = e.damaged;
        const total = d.toHealth + d.toShield;
        return {
          text: `${nameOf(d.target)} — ${$t('battleLogDamaged')} ${total}`,
          trail: d.trail,
          total,
          head: d.by == null ? '' : `${nameOf(d.by)} → ${nameOf(d.target)}`,
        };
      }
      return bare('');
    }).filter((l) => l.text),
  );

  let open = $state<number | null>(null);

  // ── Доска по высоте окна ──────────────────────────────────────────────────
  //
  // Ширина доски считается из высоты: шесть рядов клеток 3:4 дают
  // `H = 8·(W − зазоры)/3`, откуда `W = 0.375·H`. Значит, надо знать, сколько
  // высоты занято НЕ доской, — и это единственное, что нельзя написать в CSS:
  // над доской стоит шапка дома и поля страницы, а сцена о них не знает.
  //
  // Поэтому замер, а не константа. Обе величины не зависят от ширины доски
  // (руки на широком экране стоят в боковой колонке), так что обратной связи
  // нет и мерить приходится только при изменении окна.
  let fieldEl = $state<HTMLElement | null>(null);
  let tableEl = $state<HTMLElement | null>(null);
  let room = $state(272);

  function measureRoom() {
    if (!fieldEl || !tableEl) return;
    const f = fieldEl.getBoundingClientRect();
    const t = tableEl.getBoundingClientRect();
    // `+ scrollY` — отступ от верха документа: сколько окажется над доской,
    // когда страница не прокручена. Ниже доски — строка хода и воздух.
    const above = f.top + window.scrollY;
    const below = t.bottom - f.bottom;
    const next = Math.max(0, Math.round(above + below + 24));
    // Пиксель туда-сюда не стоит перерисовки доски, а стоить может кругом.
    if (Math.abs(next - room) > 2) room = next;
  }

  /** Размер окна на прошлом замере. Полоса прокрутки, появившись, шлёт `resize`
   *  и меняет `innerWidth` — а от этого меняется ширина доски, от неё высота
   *  страницы, от неё снова полоса. Круг разрывается здесь: замер идёт, только
   *  если окно правда стало другим, и только если ответ правда изменился. */
  let seen = { w: 0, h: 0 };

  onMount(() => {
    seen = { w: window.innerWidth, h: window.innerHeight };
    measureRoom();
    const again = () => {
      if (window.innerWidth === seen.w && window.innerHeight === seen.h) return;
      seen = { w: window.innerWidth, h: window.innerHeight };
      measureRoom();
    };
    window.addEventListener('resize', again);
    return () => window.removeEventListener('resize', again);
  });

  const rows = Array.from({ length: DEPTH }, (_, y) => y);
  const cols = Array.from({ length: WIDTH }, (_, x) => x);
</script>

<svelte:window onkeydown={onkey} />

<!-- Руки пишутся один раз и ставятся дважды: на широком экране — в боковую
     колонку, на узком — по краям доски. Не два куска разметки, а один снипет
     в двух местах: доска, которая рисует свою руку иначе, чем полка, начнёт
     врать ровно про то, что на ней проверяют. -->
{#snippet keeperHand()}
  {#if theirHand.length}
    <p class="hand-label">{$t('battleHandKeeper')}</p>
    <div class="hand">
      {#each theirHand as held, i (i)}
        {@const dto = dtoOf(held.name)}
        <div class="held">
          {#if dto}
            <BattleCard card={dto} {frames} owned={true} transition={false} interactive={false} />
          {:else}
            <span class="held-name">{titleOf(held.name)}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet ownHand()}
  {#if hand.length}
    <p class="hand-label">{$t('battleHandYours')}</p>
    <div class="hand">
      {#each hand as held, i (i)}
        {@const dto = dtoOf(held.name)}
        <button
          type="button"
          disabled={!mine || !playableHand.has(i)}
          onclick={() => tapHand(i)}
          class="held held--mine"
          class:held--picked={picked?.kind === 'hand' && picked.index === i}
          class:held--dim={!playableHand.has(i)}
        >
          {#if dto}
            <BattleCard card={dto} {frames} owned={true} transition={false} interactive={false} />
          {:else}
            <span class="held-name">{titleOf(held.name)}</span>
          {/if}
        </button>
      {/each}
    </div>
    <!-- Причина словами. Без неё «карта не нажимается» неотличимо от поломки. -->
    {#if handTrouble}
      <p class="hand-why">
        {handTrouble === 'mana' ? $t('battleNoManaYet') : $t('battleNoRoomYet')}
      </p>
    {/if}
  {/if}
{/snippet}

<!-- Комната и стол — разные элементы: элемент не может спрашивать свой
     собственный контейнер, и запрос ниже молча мерил бы страницу. -->
<div class="room">
<div class="scene" class:scene--held={playing}>
  <!-- Слева: круг, чей ход, мана обеих сторон. -->
  <aside class="ledger">
    <p class="ledger-line">{$t('battleRound')} {position.round}</p>
    <p class="ledger-turn">
      {position.active === me ? $t('battleWhoseTurnYours') : $t('battleWhoseTurnKeeper')}
    </p>
    <!--
      «Сколько есть сейчас» имеет смысл только у той стороны, чей ход идёт. У
      другой `mana` — это остаток с её прошлого хода, а в первом раунде, пока
      она ещё не ходила ни разу, это просто ноль при непустом потолке: сторона,
      ходящая второй, показывала «0/2», а на своём первом ходу получала 3.
      Число, которое ни разу не было правдой.

      Поэтому у активной стороны показывается «есть из потолка», а у неактивной
      — только сам потолок. Дорисовывать ей «будет столько-то» здесь нельзя:
      прибавку на ход считает движок, и вторая её реализация разошлась бы с ним.
    -->
    <p class="ledger-line">
      {$t('battleManaYours')}
      <span class="num">
        {#if position.active === 'player'}{position.player.mana}/{position.player.manaMax}
        {:else}{position.player.manaMax}{/if}
      </span>
    </p>
    <p class="ledger-line">
      {$t('battleManaKeeper')}
      <span class="num">
        {#if position.active === 'keeper'}{position.keeper.mana}/{position.keeper.manaMax}
        {:else}{position.keeper.manaMax}{/if}
      </span>
    </p>
    <p class="ledger-note">{$t('battleManaNote')}</p>

    <!-- Обе руки, лицом: этюд решают рассуждением. Порядок сверху вниз тот же,
         что на доске, — хранитель над гостем. -->
    <div class="ledger-hands">
      {@render keeperHand()}
      {@render ownHand()}
    </div>
  </aside>

  <div class="table" bind:this={tableEl} style="--room:{room}px">
    <div class="table-hand table-hand--theirs">{@render keeperHand()}</div>

    <div class="field" bind:this={fieldEl}>
      <div class="grid">
        {#each rows as y (y)}
          {#each cols as x (x)}
            {@const here = unitAt(x, y)}
            {@const open2 = openCells.has(`${x},${y}`)}
            {@const target = here ? openUnits.get(here.id) : undefined}
            {@const dto = here ? dtoOf(here.card.name) : null}
            <button
              type="button"
              disabled={!mine}
              onclick={() => tapCell(x, y)}
              aria-label={here ? titleOf(here.card.name) : `${x},${y}`}
              class="cell"
              class:cell--open={open2}
              class:cell--picked={picked?.kind === 'unit' && here?.id === picked.id}
              class:cell--attack={target === 'attack'}
              class:cell--mend={target === 'mend'}
              class:cell--live={mine && here?.owner === me && ready.has(here.id)}
            >
              {#if here}
                <!-- Погасшим показывается и тело, которое уже сходило, и тело,
                     которому нечем ходить: с тех пор как шаг не тратит ход
                     целиком, второе случается часто, и без этого оно выглядело
                     бы свежим, не отзываясь на нажатие. -->
                <span
                  class="figure"
                  style:--fit={dto ? aspectOf(dto) : DEFAULT_ASPECT}
                  class:figure--spent={here.owner === me
                    && (here.acted || (mine && !ready.has(here.id)))}
                  class:figure--falling={falling === here.id}
                  class:figure--flinch={flinch === here.id}
                  style={lunge?.unit === here.id
                    ? `--lx:${lunge.dx}%;--ly:${lunge.dy}%`
                    : undefined}
                  class:figure--lunge={lunge?.unit === here.id}
                >
                  {#if dto}
                    <BattleCard card={dto} {frames} owned={true} transition={false} interactive={false} />
                  {:else}
                    <span class="figure-name">{titleOf(here.card.name)}</span>
                  {/if}
                </span>

                <!-- Число — только у раненых. Целое тело сообщает о себе тем,
                     что молчит. -->
                {#if here.health.current < here.health.max}
                  <span class="wound">{here.health.current}</span>
                {/if}

                {#if here.statuses.length}
                  <span class="nicks" aria-hidden="true">
                    {#each here.statuses as st, i (i)}<i class="nick"></i>{/each}
                  </span>
                {/if}
              {/if}
            </button>
          {/each}
        {/each}
        <span class="midline" aria-hidden="true"></span>
      </div>

      <!-- Следы ударов. viewBox под сетку клеток: линия декоративна, и доли
           пикселя, которые съедают зазоры, ей ничего не стоят. -->
      <svg class="traces" viewBox="0 0 {WIDTH} {DEPTH}" preserveAspectRatio="none" aria-hidden="true">
        {#each traces as tr (tr.key)}
          <line
            class="trace"
            class:trace--fresh={tr.fresh}
            x1={tr.from.x + 0.5} y1={tr.from.y + 0.5}
            x2={tr.to.x + 0.5} y2={tr.to.y + 0.5}
            vector-effect="non-scaling-stroke"
          />
          <line
            class="trace-grip"
            x1={tr.from.x + 0.5} y1={tr.from.y + 0.5}
            x2={tr.to.x + 0.5} y2={tr.to.y + 0.5}
            vector-effect="non-scaling-stroke"
            role="presentation"
            onmouseenter={() => (reading = tr.key)}
            onmouseleave={() => (reading = null)}
          />
        {/each}
      </svg>

      {#if reading !== null}
        {@const tr = traces.find((x) => x.key === reading)}
        {#if tr}
          <div
            class="breakdown"
            style="--bx:{((tr.from.x + tr.to.x + 1) / 2 / WIDTH) * 100}%;--by:{((tr.from.y + tr.to.y + 1) / 2 / DEPTH) * 100}%"
          >
            <p class="breakdown-head">{tr.by} → {tr.at}</p>
            {#if tr.immune}
              <p class="breakdown-row"><span>{$t('battleStepImmunity')}</span></p>
            {:else}
              {#each tr.trail as b, i (i)}
                <p class="breakdown-row">
                  <span class="breakdown-why">{stepWord(b.step, $t)}</span>
                  <span class="num">{b.from} → {b.to}</span>
                </p>
              {/each}
              <p class="breakdown-row breakdown-total">
                <span class="breakdown-why">{$t('battleTrailTotal')}</span>
                <span class="num">{tr.total}</span>
              </p>
            {/if}
          </div>
        {/if}
      {/if}

      <!-- Исход. Сургучная печать, тот же приём, что при покупке карты. -->
      {#if position.outcome && !playing}
        <div class="seal-wrap" transition:fade={{ duration: 200 }}>
          <div class="seal" class:seal--dim={position.outcome !== 'player'}>
            <p class="seal-word">
              {position.outcome === 'player'
                ? $t('battleWonByPlayer')
                : position.outcome === 'keeper'
                  ? $t('battleWonByKeeper')
                  : $t('battleDrawn')}
            </p>
            {#if match.rewardDust > 0}
              <p class="seal-dust">{match.rewardDust} {$t('battleDustGranted')}</p>
            {/if}
            {#if onleave || onreplay}
              <p class="seal-doors">
                {#if onleave}<button type="button" class="door" onclick={onleave}>{$t('battleBackToStudies')}</button>{/if}
                {#if onreplay}<button type="button" class="door" onclick={onreplay}>{$t('battleReplay')}</button>{/if}
              </p>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Своя рука, ближе к рукам: на телефоне это буквально так. -->
    <div class="table-hand table-hand--mine">{@render ownHand()}</div>

    <div class="turn">
      <button type="button" disabled={!mine} onclick={() => onact('endTurn')} class="end">
        {$t('battleEndTurn')}
      </button>
      {#if busy || playing}
        <span class="waiting">{$t('battleKeeperThinks')}</span>
      {/if}
    </div>
  </div>

  <aside class="aside">
    <!-- Карточка выбранного: спокойная и неподвижная, не всплывающая подсказка. -->
    {#if chosen}
      <div class="chosen">
        <p class="chosen-name">{titleOf(chosen.card.name)}</p>
        <dl class="chosen-stats">
          <div><dt>{$t('battlesHealthLabel')}</dt><dd class="num">{chosen.health.current}/{chosen.health.max}</dd></div>
          <div><dt>{$t('battlesPowerLabel')}</dt><dd class="num">{chosen.power}</dd></div>
          <div><dt>{$t('battleStatArmour')}</dt><dd class="num">{chosen.armor}</dd></div>
          <div><dt>{$t('battleStatWard')}</dt><dd class="num">{chosen.ward}</dd></div>
          <div><dt>{$t('battleStatReach')}</dt><dd class="num">{chosen.reach}</dd></div>
          <div><dt>{$t('battleStatStep')}</dt><dd class="num">{chosen.step}</dd></div>
          {#if chosen.mend > 0}
            <div><dt>{$t('battleStatMend')}</dt><dd class="num">{chosen.mend}</dd></div>
          {/if}
        </dl>
        {#if chosen.statuses.length}
          <ul class="chosen-riders">
            {#each chosen.statuses as st, i (i)}
              <li>{st.name} <span class="num">{st.amount}</span> · {$t('battleStatusTurns')} {st.turns}</li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    <div class="journal">
      <p class="journal-label">{$t('battleJournal')}</p>
      {#if !journal.length}
        <p class="journal-empty">{$t('battleJournalEmpty')}</p>
      {:else}
        <ul class="journal-lines">
          {#each journal as line, i (i)}
            <li>
              {#if line.trail.length}
                <button type="button" class="journal-open" onclick={() => (open = open === i ? null : i)}>
                  {line.text}
                </button>
                {#if open === i}
                  <div class="breakdown breakdown--flat">
                    {#if line.head}<p class="breakdown-head">{line.head}</p>{/if}
                    {#each line.trail as b, j (j)}
                      <p class="breakdown-row">
                        <span class="breakdown-why">{stepWord(b.step, $t)}</span>
                        <span class="num">{b.from} → {b.to}</span>
                      </p>
                    {/each}
                    <p class="breakdown-row breakdown-total">
                      <span class="breakdown-why">{$t('battleTrailTotal')}</span>
                      <span class="num">{line.total}</span>
                    </p>
                  </div>
                {/if}
              {:else}
                <span class="journal-plain">{line.text}</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </aside>
</div>
</div>

<style>
  .room {
    container-type: inline-size;
  }

  .scene {
    display: flex;
    flex-direction: column;
    gap: 1.6rem;
    color: #34251c;
  }

  /* Три колонки только там, где они помещаются. Стол хранителя рисует ту же
     сцену в 24rem, и там она складывается в одну колонку сама. */
  @container (min-width: 900px) {
    .scene {
      display: grid;
      grid-template-columns: 13rem minmax(0, 26rem) minmax(0, 1fr);
      align-items: start;
      gap: 2rem;
    }

    /* Руки уходят в боковую колонку — и только ради этого доска помещается в
       окно целиком. Две руки картами стоят двести с лишним пикселей высоты,
       а поле 3×6 и без них выше экрана: держать и то и другое в одной колонке
       значит либо прокручивать доску, либо ужать карту до пятна. */
    /* Через `.scene`, а не просто `.ledger-hands`: контейнерный запрос не
       добавляет веса, а стоит он в этом файле выше общего правила — значит,
       перебить его может только вес. */
    .scene .ledger-hands {
      display: contents;
    }

    .table-hand {
      display: none;
    }

    /* Доска — по высоте окна: ширина из высоты (`W = 0.375·H`), запас замерен
       выше. Пол в 13rem не даёт доске ужаться в невидимое: на низком окне
       честнее прокрутить, чем разглядывать фотографию величиной с ноготь.
       Ширина считается один раз и достаётся полю целиком: следы ударов и
       печать лежат в `inset: 0` от поля, и поле шире доски увело бы их вбок. */
    .table {
      --board: clamp(13rem, calc((100dvh - var(--room, 17rem) - 20px) * 0.375 + 8px), 26rem);
    }

    .field,
    .turn {
      width: var(--board);
      margin-inline: auto;
    }
  }

  .ledger {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .ledger-turn {
    color: #6f3b24;
  }

  /* На узком экране руки остаются у краёв доски, как задумано: своя половина
     ближе к рукам, и на телефоне это буквально так. */
  .ledger-hands {
    display: none;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .table {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
    /* Доска — колонка около 420 px, и на широком экране тоже: поле 3×6
       вертикально само по себе, растягивать его некуда. */
    max-width: 26rem;
  }

  .field {
    position: relative;
  }

  .grid {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }

  /* Черта между половинами. Абсолютная, а не отступ ряда: сетка должна остаться
     ровной, иначе следы ударов поедут вместе с ней. */
  .midline {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    border-top: 1px dashed rgba(52, 37, 28, 0.2);
    pointer-events: none;
  }

  .cell {
    position: relative;
    aspect-ratio: 3 / 4;
    border: 1px solid rgba(52, 37, 28, 0.12);
    background: transparent;
    text-align: left;
    transition: background-color 200ms ease, border-color 200ms ease;
  }

  /* Куда можно шагнуть: клетка светлеет на тон, кромка становится сплошной. */
  .cell--open {
    background: rgba(111, 59, 36, 0.06);
    border-color: rgba(52, 37, 28, 0.4);
  }

  .cell--live {
    cursor: pointer;
  }

  /* Выбранное: обводка по кромке КЛЕТКИ — кромка карты занята рамкой чина. */
  .cell--picked {
    border-color: #c65f3c;
  }

  .cell--live:hover .figure,
  .cell--picked .figure {
    transform: translateY(-2px);
    filter: drop-shadow(0 4px 10px rgba(52, 37, 28, 0.28));
  }

  /* Цель: подложка под картой, выступающая на два пиксела. Сплошная — удар,
     прерывистая — лечение. Ни зелёного, ни красного, и разница не в цвете. */
  .cell--attack::before,
  .cell--mend::before {
    content: '';
    position: absolute;
    inset: 1px;
    pointer-events: none;
  }

  .cell--attack::before {
    background: #6f3b24;
  }

  .cell--mend::before {
    border: 2px dashed #6f3b24;
  }

  .figure {
    position: relative;
    z-index: 1;
    display: grid;
    place-items: center;
    height: 100%;
    padding: 3px;
    transition:
      transform 180ms cubic-bezier(0.2, 0.8, 0.25, 1),
      opacity 500ms ease,
      filter 180ms ease;
  }

  /* Ходил — приглушение, не слово. */
  /* Каждая рамка носит своё отношение сторон, а клетки одинаковы. Ширина
     считается, а не выводится из содержимого: клетка 3:4 (см. `.cell`), значит
     её высота — 133⅓% ширины, и карта ростом в клетку шириной `133⅓% × отношение`.
     Шире клетки она не станет — тогда её держит `100%`, а высоту досчитает
     собственный `aspect-ratio`.

     Так, а не `height: 100%; width: auto`: карта — контейнер (`container-type`),
     и её содержимое меряется в `cqi`. Позволить такому боксу мерить свою ширину
     по содержимому — значит замкнуть круг: ширина из содержимого, содержимое из
     ширины. Широкая рамка в этом круге вырастала в полтора раза больше клетки и
     ложилась поверх соседей. Процент от клетки круг разрывает. */
  .figure > :global(.slot) {
    width: min(100%, calc(133.3333% * var(--fit, 0.714)));
  }

  .figure--spent {
    opacity: 0.55;
  }

  .figure--lunge {
    transform: translate(var(--lx, 0), var(--ly, 0));
    transition: transform 220ms cubic-bezier(0.2, 0.8, 0.25, 1);
  }

  .figure--flinch {
    animation: flinch 160ms ease;
  }

  @keyframes flinch {
    0% { transform: translate(0, 0); }
    40% { transform: translate(3px, 0); }
    100% { transform: translate(0, 0); }
  }

  /* Бледнеет и оседает. Ни черепа, ни крестика, ни слова «убит». */
  .figure--falling {
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 500ms ease, transform 500ms ease;
  }

  .figure-name,
  .held-name {
    display: block;
    padding: 0.3rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 12px;
    line-height: 1.2;
  }

  .wound {
    position: absolute;
    left: 4px;
    bottom: 4px;
    z-index: 2;
    padding: 0 3px;
    background: #f8f1e7;
    border: 1px solid rgba(52, 37, 28, 0.25);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: #34251c;
  }

  .nicks {
    position: absolute;
    right: 2px;
    top: 5px;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nick {
    display: block;
    width: 5px;
    height: 2px;
    background: #6f3b24;
  }

  .traces {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 3;
    pointer-events: none;
  }

  .trace {
    stroke: #6f3b24;
    stroke-width: 1;
    opacity: 0.25;
    transition: opacity 400ms ease;
  }

  .trace--fresh {
    opacity: 0.85;
  }

  /* Широкая невидимая жила: наводиться на волосяную линию невозможно. */
  .trace-grip {
    stroke: transparent;
    stroke-width: 14;
    pointer-events: stroke;
    cursor: help;
  }

  /* На узком экране наведения нет, а касание уже занято снятием выбора:
     разбор там берут из журнала. */
  @media (hover: none) {
    .trace-grip {
      pointer-events: none;
    }
  }

  .breakdown {
    position: absolute;
    left: var(--bx, 50%);
    top: var(--by, 50%);
    z-index: 5;
    transform: translate(-50%, -50%);
    min-width: 14rem;
    padding: 0.6rem 0.75rem;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    box-shadow: 0 3px 16px rgba(52, 37, 28, 0.18);
    font-size: 12px;
    line-height: 1.5;
    pointer-events: none;
  }

  .breakdown--flat {
    position: static;
    transform: none;
    min-width: 0;
    margin: 0.3rem 0 0.5rem;
    box-shadow: none;
  }

  .breakdown-head {
    margin-bottom: 0.3rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
  }

  .breakdown-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    color: #5f4636;
  }

  .breakdown-total {
    margin-top: 0.25rem;
    padding-top: 0.25rem;
    border-top: 1px solid rgba(52, 37, 28, 0.15);
    color: #34251c;
  }

  .hand {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .hand-label {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .held {
    display: block;
    width: 4.5rem;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
  }

  .held--mine {
    cursor: pointer;
  }

  .held--picked {
    border-color: #c65f3c;
  }

  /* Выложить нельзя — и это должно читаться сразу, а не угадываться. Было
     0.4 без каких-либо других отличий, и карта выглядела просто бледной. */
  .held--dim {
    opacity: 0.32;
    filter: grayscale(0.7);
    cursor: default;
  }

  .ledger-note {
    margin: 0.35rem 0 0;
    max-width: 30ch;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    font-style: italic;
    line-height: 1.5;
    color: #8a6a55;
  }

  .hand-why {
    margin: 0.5rem 0 0;
    max-width: 30ch;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.82rem;
    font-style: italic;
    line-height: 1.5;
    color: #8a6a55;
  }

  .turn {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .end {
    font-family: inherit;
    padding: 0.5rem 1rem;
    border: 1px solid rgba(52, 37, 28, 0.25);
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .end:disabled {
    opacity: 0.4;
  }

  .end:not(:disabled):hover {
    background: rgba(52, 37, 28, 0.05);
  }

  .waiting {
    font-size: 11px;
    color: #8a6a55;
  }

  /* Пока журнал играет, доска не принимает касаний, и курсор обычный. */
  .scene--held .cell,
  .scene--held .held {
    cursor: default;
  }

  .aside {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    min-width: 0;
    font-size: 12px;
  }

  .chosen {
    padding: 0.75rem;
    border: 1px solid #d8c6b1;
  }

  .chosen-name {
    margin-bottom: 0.4rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 16px;
  }

  .chosen-stats {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.15rem 1rem;
    color: #5f4636;
  }

  .chosen-stats div {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .chosen-stats dt {
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .chosen-riders {
    margin-top: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
    color: #5f4636;
  }

  .journal-label {
    margin-bottom: 0.4rem;
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .journal-empty {
    font-style: italic;
    color: #8a6a55;
  }

  .journal-lines {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    color: #5f4636;
    line-height: 1.5;
  }

  .journal-open {
    font: inherit;
    text-align: left;
    border-bottom: 1px dotted rgba(52, 37, 28, 0.3);
    cursor: pointer;
  }

  .journal-open:hover {
    color: #c65f3c;
  }

  /* ── Печать ────────────────────────────────────────────────────────────── */

  .seal-wrap {
    position: absolute;
    inset: 0;
    z-index: 6;
    background: rgba(248, 241, 231, 0.82);
  }

  /* Поле 3×6 выше окна, поэтому печать держится в виду, а не в середине доски:
     оттиск, за которым надо прокручивать, — это не оттиск. */
  .seal {
    position: sticky;
    top: max(1.5rem, calc(50vh - 7rem));
    width: max-content;
    max-width: 100%;
    margin: 0 auto;
    padding: 1.5rem 1.75rem;
    text-align: center;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 4px;
    box-shadow: 0 6px 28px rgba(52, 37, 28, 0.2);
    animation: press 600ms cubic-bezier(0.2, 0.8, 0.25, 1) both;
  }

  /* Поле за хранителем — та же печать, но тусклее и без поворота. */
  .seal--dim {
    opacity: 0.8;
    animation: press-flat 600ms ease both;
  }

  @keyframes press {
    from { transform: scale(1.14) rotate(-4deg); opacity: 0; }
    to { transform: scale(1) rotate(-1deg); opacity: 1; }
  }

  @keyframes press-flat {
    from { transform: scale(1.1); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }

  .seal-word {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 20px;
    color: #34251c;
  }

  .seal-dust {
    margin-top: 0.4rem;
    font-size: 12px;
    color: #6f3b24;
  }

  .seal-doors {
    display: flex;
    gap: 1rem;
    justify-content: center;
    margin-top: 1rem;
  }

  .door {
    font-family: inherit;
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
    cursor: pointer;
  }

  .door:hover {
    color: #c65f3c;
  }

  @media (prefers-reduced-motion: reduce) {
    .figure,
    .cell,
    .trace {
      transition: none;
    }

    .figure--flinch {
      animation: none;
    }

    .seal,
    .seal--dim {
      animation: none;
    }
  }
</style>
