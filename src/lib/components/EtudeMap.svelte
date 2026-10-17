<script lang="ts">
  // Карта этюдов.
  //
  // Слои снизу вверх, и деление их — замер, а не вкус:
  // 1. картинка: рельеф, реки, озёра, леса — считается в отдельном потоке и
  //    лежит в кэше браузера (`loadEtudeMap`); на странице это одна <img>;
  // 2. неподвижный SVG: дорога, картуш, легенда, рамка — после вступления в
  //    нём ничего не движется, и он не перерисовывается;
  // 3. движущийся SVG: вода в озёрах, волны, змей, корабль, птицы, роза
  //    ветров — свой слой, чтобы их кадры не перерисовывали остальное;
  // 4. облака — картинками, двигаются сдвигом;
  // 5. знаки этюдов и подписи;
  // 6. свет за указателем — пятно, которое двигается сдвигом.
  // Бесконечные анимации идут не с частотой экрана, а тиком (`PACE_MS`).
  // Пока деревья были элементами SVG рядом с анимациями, карта в покое
  // держала почти целое ядро процессора, а движение мыши пересчитывало стили
  // восьми с половиной тысяч элементов на каждом кадре.
  //
  // Этюд открывается листком у своего знака. Что в листке — решает страница
  // (`sheet`): правила этюда и кнопка партии живут там же, где партия.
  //
  // При `prefers-reduced-motion` карта стоит готовой: ни разворота, ни
  // облаков, ни корабля, ни света за указателем.
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { MAP_H, MAP_W, layRoad, roman, type MapMotion } from '$lib/etudeMap';
  import { loadEtudeMap, type LoadedMap } from '$lib/etudeMapLoad';
  import type { BattleChallenge } from '$lib/types/api';
  import EtudeGlyph from './EtudeGlyph.svelte';

  export type EtudeKind = 'study' | 'meeting' | 'city';

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

  // Дорога не зависит от числа этюдов, знаки на ней — зависят.
  let road = $derived(layRoad(challenges.length));
  let loaded = $state<LoadedMap | null>(null);
  let motion = $derived<MapMotion | null>(loaded?.motion ?? null);
  let still = $state(false);
  /** Карта ушла за край экрана — движение стоит. */
  let away = $state(false);

  let mapEl = $state<HTMLDivElement>();
  let sheetEl = $state<HTMLDivElement>();
  let lightEl = $state<HTMLDivElement>();
  let cloudsEl = $state<HTMLDivElement>();
  let motionSvg = $state<SVGSVGElement>();
  let scroller = $state<HTMLDivElement>();

  let openId = $state<string | null>(null);
  let opened = $derived(challenges.find((c) => c.id === openId) ?? null);
  let openedIndex = $derived(opened ? challenges.indexOf(opened) : -1);

  /** Следующий бой — первый не выигранный по порядку дороги. */
  let current = $derived.by(() => {
    const i = challenges.findIndex((c) => !c.marks?.won);
    return i < 0 ? challenges.length - 1 : i;
  });
  /** Пройденная часть дороги — до следующего боя. */
  let travel = $derived(
    challenges.length === 0 ? 0 : challenges.every((c) => c.marks?.won) ? 1 : (road.pins[current]?.t ?? 0),
  );

  /** Подпись в одну или две строки: в SVG текст сам не переносится. */
  function labelLines(text: string): string[] {
    if (text.length <= 22) return [text];
    const words = text.split(' ');
    let best = [text];
    let gap = Infinity;
    for (let i = 1; i < words.length; i++) {
      const a = words.slice(0, i).join(' ');
      const b = words.slice(i).join(' ');
      const d = Math.abs(a.length - b.length);
      if (d < gap) {
        gap = d;
        best = [a, b];
      }
    }
    return best;
  }

  onMount(() => {
    still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    let alive = true;
    let held: LoadedMap | null = null;
    void loadEtudeMap(challenges.length).then((map) => {
      if (!alive) {
        map.release();
        return;
      }
      held = map;
      loaded = map;
    });
    // Карта за краем экрана движение не показывает и процессор не тратит.
    const watch = new IntersectionObserver(([entry]) => (away = !entry.isIntersecting));
    if (mapEl) watch.observe(mapEl);
    return () => {
      alive = false;
      watch.disconnect();
      held?.release();
    };
  });

  /**
   * Темп движения на карте — шестнадцать шагов в секунду, а не частота
   * экрана. Пока на странице идёт хоть одна непрерывная анимация, браузер
   * собирает 60–120 кадров в секунду, и одни облака держали так треть ядра.
   * Поэтому бесконечные анимации стоят на паузе, а этот тик сам переводит их
   * время вперёд: между шагами перерисовывать нечего. Вступление (разворот,
   * дорога, знаки) конечно и идёт обычным ходом. Замер: в покое весь Chrome
   * с картой — 81 % ядра до, около 30 % после.
   */
  const PACE_MS = 1000 / 16;
  $effect(() => {
    const map = mapEl;
    const svg = motionSvg;
    if (!map || !loaded || still || away) return;
    const held = new Map<Animation, number>();
    svg?.pauseAnimations();
    const smilFrom = svg?.getCurrentTime() ?? 0;
    const t0 = performance.now();
    const step = () => {
      if (document.hidden) return;
      const dt = performance.now() - t0;
      // Новые анимации появляются и после начала: облака, свет у
      // следующего знака. Их берём на том же шаге.
      for (const a of map.getAnimations({ subtree: true })) {
        if (held.has(a) || a.effect?.getComputedTiming().iterations !== Infinity) continue;
        held.set(a, Number(a.currentTime ?? 0) - dt);
        a.pause();
      }
      for (const [a, from] of held) a.currentTime = from + dt;
      svg?.setCurrentTime(smilFrom + dt / 1000);
    };
    step();
    const timer = setInterval(step, PACE_MS);
    return () => {
      clearInterval(timer);
      // Карта ушла за край экрана: всё замирает на месте до возвращения.
      for (const a of held.keys()) a.pause();
    };
  });

  // На узком экране карта шире окна: подвести к следующему бою.
  $effect(() => {
    const pin = road.pins[current];
    if (!scroller || !mapEl || !pin) return;
    if (scroller.scrollWidth <= scroller.clientWidth) return;
    const x = (pin.x / MAP_W) * mapEl.clientWidth - scroller.clientWidth / 2;
    scroller.scrollLeft = Math.max(0, x);
  });

  // Свет, наклон и облака за указателем пишутся прямо в стиль трёх
  // элементов. Не переменными на карте: переменная наследуется, и браузер
  // пересчитывал стили всех её потомков на каждом движении мыши.
  let frame = 0;
  function onpointermove(e: PointerEvent) {
    if (still || e.pointerType === 'touch' || !mapEl) return;
    const box = mapEl.getBoundingClientRect();
    const x = (e.clientX - box.left) / box.width;
    const y = (e.clientY - box.top) / box.height;
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      if (sheetEl) {
        sheetEl.style.transform = `perspective(2200px) rotateX(${((0.5 - y) * 1.6).toFixed(3)}deg) rotateY(${((x - 0.5) * 2.2).toFixed(3)}deg)`;
      }
      if (lightEl) {
        lightEl.style.transform = `translate(${(x * box.width).toFixed(0)}px, ${(y * box.height).toFixed(0)}px)`;
        lightEl.style.opacity = '1';
      }
      if (cloudsEl) {
        cloudsEl.style.transform = `translate(${((0.5 - x) * 18).toFixed(1)}px, ${((0.5 - y) * 12).toFixed(1)}px)`;
      }
    });
  }
  function onpointerleave() {
    cancelAnimationFrame(frame);
    if (sheetEl) sheetEl.style.transform = '';
    if (lightEl) lightEl.style.opacity = '0';
    if (cloudsEl) cloudsEl.style.transform = '';
  }

  function openPin(c: BattleChallenge) {
    openId = openId === c.id ? null : c.id;
  }

  function onPinKey(e: KeyboardEvent, c: BattleChallenge) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openPin(c);
    }
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && openId) {
      openId = null;
      e.stopPropagation();
    }
  }

  /** Щелчок мимо листка и мимо знаков закрывает листок. */
  function onpointeraway(e: PointerEvent) {
    if (!openId) return;
    const target = e.target as Element | null;
    if (target?.closest('.leaf, .pin-hit')) return;
    openId = null;
  }

  /** Листок встаёт сбоку от знака, к середине карты. */
  let sheetPlace = $derived.by(() => {
    if (openedIndex < 0) return '';
    const pin = road.pins[openedIndex];
    if (!pin) return '';
    const left = pin.x / MAP_W;
    const top = pin.y / MAP_H;
    const sx = left > 0.58 ? 'calc(-100% - 3.6rem)' : '3.6rem';
    const sy = top > 0.62 ? '-85%' : top < 0.3 ? '-10%' : '-45%';
    return `left:${(left * 100).toFixed(2)}%;top:${(top * 100).toFixed(2)}%;--sx:${sx};--sy:${sy}`;
  });

  const BIRDS = [
    { path: 'M-80 330C300 250 760 420 1180 260S1600 180 1700 210', dur: 62, begin: 0 },
    { path: 'M1700 620C1300 520 980 690 620 560S200 470 -80 520', dur: 74, begin: 21 },
  ];
</script>

<svelte:window onkeydown={onkey} onpointerdown={onpointeraway} />

<div class="scroll" bind:this={scroller}>
  <div
    class="map"
    class:map--still={still}
    class:map--painted={!!loaded}
    bind:this={mapEl}
    {onpointermove}
    {onpointerleave}
    role="presentation"
  >
    <div class="sheet" bind:this={sheetEl}>
      <div class="paper" aria-hidden="true"></div>
      {#if loaded}
        <img class="picture" src={loaded.picture} alt="" aria-hidden="true" />
      {:else}
        <!-- Первый расчёт карты занимает секунды; пустая бумага за это время
             читается как сломанная страница. -->
        <p class="drawing" role="status">{$t('battleMapDrawing')}</p>
      {/if}

      <!-- Неподвижное: дорога, картуш, легенда, рамка -->
      <svg class="layer" viewBox="0 0 {MAP_W} {MAP_H}" aria-hidden="true">
        <defs>
          <mask id="etude-road-walked" maskUnits="userSpaceOnUse">
            <path
              class="walk"
              d={road.d}
              pathLength="1"
              fill="none"
              stroke="#fff"
              stroke-width="16"
              style="--walk:{travel}"
            />
          </mask>
        </defs>

        <!-- Дорога: вся — пунктиром, пройденная — чернилами поверх. -->
        <path class="road road--bed" d={road.d} />
        <path class="road road--ahead" d={road.d} />
        <path class="road road--walked" d={road.d} mask="url(#etude-road-walked)" />

        <!-- Картуш. Вдвое меньше прежнего и без «Карточная игра»: где
             человек находится, уже сказала строка навигации над картой, а
             угол карты нужнее местности. -->
        <g class="cartouche" transform="translate(40 36)">
          <path class="ct-scroll" d="M12 0H218q12 0 12 12V60q0 12 -12 12H12q-12 0 -12 -12V12Q0 0 12 0Z" />
          <path class="ct-scroll-inner" d="M18 6H212q8 0 8 8V58q0 8 -8 8H18q-8 0 -8 -8V14q0 -8 8 -8Z" />
          <text x="115" y="48" class="ct-title">{$t('battleStudies')}</text>
          <path class="ct-rule" d="M70 58H160M108 58l7 -3l7 3l-7 3z" />
        </g>

        <!-- Рамка с градусной лентой -->
        <rect x="10" y="10" width={MAP_W - 20} height={MAP_H - 20} class="frame frame--outer" />
        <rect x="16" y="16" width={MAP_W - 32} height={MAP_H - 32} class="frame frame--band" />
        <rect x="22" y="22" width={MAP_W - 44} height={MAP_H - 44} class="frame frame--inner" />
      </svg>

      <!-- Движущееся: свой слой, чтобы кадры не перерисовывали остальное -->
      <svg class="layer layer--motion" viewBox="0 0 {MAP_W} {MAP_H}" aria-hidden="true" bind:this={motionSvg}>
        <defs>
          <linearGradient id="etude-sheen" x1="0" y1="0" x2="1" y2="0.35">
            <stop offset="0.3" stop-color="#fff8e6" stop-opacity="0" />
            <stop offset="0.5" stop-color="#fff8e6" stop-opacity="0.55" />
            <stop offset="0.7" stop-color="#fff8e6" stop-opacity="0" />
          </linearGradient>
        </defs>

        {#if motion}
          {#each motion.lakes as lake, i (i)}
            <g class="lake">
              <clipPath id="etude-lake-{i}"><path d={lake.d} /></clipPath>
              <g clip-path="url(#etude-lake-{i})">
                <rect
                  class="lake-sheen"
                  x={lake.box.x}
                  y={lake.box.y}
                  width={lake.box.w}
                  height={lake.box.h}
                  fill="url(#etude-sheen)"
                />
                {#each lake.hatch as line, k (k)}
                  <path class="lake-hatch lake-hatch--{k}" d={line} />
                {/each}
                {#each lake.ripples as w, k (k)}
                  <path class="lake-ripple" d="M{w.x - 8} {w.y}q4 -2.6 8 0t8 0" style="--d:{w.delay}s" />
                {/each}
                {#each lake.glints as g, k (k)}
                  <path
                    class="lake-glint"
                    d="M{g.x - g.len / 2} {g.y}h{g.len}"
                    style="--d:{g.delay}s;--t:{g.dur}s"
                  />
                {/each}
              </g>
              <g class="reeds">
                <path class="reed" d={lake.reeds} />
                <path class="reed-head" d={lake.heads} />
              </g>
            </g>
          {/each}

          <g class="waves">
            {#each motion.waves as w, i (i)}
              <path
                class="wave"
                d="M{w.x - 9} {w.y}q4.5 -4 9 0t9 0M{w.x - 3} {w.y + 6}q3 -2.6 6 0"
                style="--d:{(i % 7) * -0.9}s"
              />
            {/each}
          </g>

          {#if motion.serpent}
            <g class="serpent" transform="translate({motion.serpent.x} {motion.serpent.y})">
              <path class="s-water" d="M-62 8q8 -4 16 0t16 0t16 0t16 0t16 0t16 0t16 0" />
              <path class="s-hump s-hump--1" d="M-48 6c2 -22 22 -22 24 0" />
              <path class="s-hump s-hump--2" d="M-16 6c2 -26 24 -26 26 0" />
              <path class="s-hump s-hump--3" d="M18 6c1 -16 16 -16 17 0" />
              <g class="s-head">
                <path
                  class="s-neck"
                  d="M-58 6c-6 -18 -4 -34 8 -40c10 -5 20 0 22 6c1 4 -3 7 -8 5c-6 -3 -10 2 -9 8c1 8 4 14 4 21"
                />
                <circle cx="-44" cy="-29" r="1.6" class="s-eye" />
                <path class="s-fin" d="M-52 -18l-9 -4l7 8M-55 -6l-9 -2l8 6" />
              </g>
              <path class="s-tail" d="M44 6c4 -10 12 -12 18 -6c-6 -1 -10 2 -12 6" />
            </g>
          {/if}

          {#if motion.ship && !still}
            <g class="ship">
              <g class="ship-bob">
                <path class="ship-wake" d="M18 6q-10 2 -22 0M14 9q-8 2 -18 1" />
                <path class="ship-hull" d="M-17 0h34l-6 8h-22z" />
                <path class="ship-mast" d="M0 0v-30M-9 -4v-18" />
                <path class="ship-sail" d="M1 -28q12 10 0 22zM-8 -21q8 7 0 15z" />
                <path class="ship-pennant" d="M0 -30l9 2l-9 3" />
              </g>
              <animateMotion dur="140s" repeatCount="indefinite" rotate="0" path={motion.ship} />
            </g>
          {/if}
        {/if}

        {#if !still}
          {#each BIRDS as flock, f (f)}
            <g class="flock">
              {#each [0, 1, 2, 3] as b (b)}
                <path
                  class="bird"
                  d="M{-6 + b * 14} {(b % 2) * 9}q3 -4 6 0q3 -4 6 0"
                  style="--d:{b * 0.17}s"
                />
              {/each}
              <animateMotion dur="{flock.dur}s" begin="{flock.begin}s" repeatCount="indefinite" path={flock.path} />
            </g>
          {/each}
        {/if}

        <!-- Роза ветров -->
        <g class="compass" transform="translate(1455 865)">
          <circle r="78" class="c-ring" />
          <circle r="70" class="c-ring c-ring--thin" />
          <circle r="70" class="c-ticks" />
          <g class="c-needle">
            {#each [0, 45, 90, 135, 180, 225, 270, 315] as a (a)}
              <g transform="rotate({a})">
                <path
                  class={a % 90 === 0 ? 'c-point' : 'c-point c-point--minor'}
                  d={a % 90 === 0 ? 'M0 -66L9 -9L0 0Z' : 'M0 -44L6 -6L0 0Z'}
                />
                <path
                  class="c-point-dark"
                  d={a % 90 === 0 ? 'M0 -66L-9 -9L0 0Z' : 'M0 -44L-6 -6L0 0Z'}
                />
              </g>
            {/each}
            <circle r="5" class="c-hub" />
          </g>
          <text y="-84" class="c-north">N</text>
        </g>
      </svg>

      {#if loaded && loaded.clouds.length && !still}
        <div class="clouds" aria-hidden="true" bind:this={cloudsEl}>
          {#each loaded.clouds as src, i (i)}
            <div class="drift drift--{i}">
              <img class="cloud-shadow" {src} alt="" />
              <img class="cloud" {src} alt="" />
            </div>
          {/each}
        </div>
      {/if}
      <!-- Этюды -->
      <svg class="layer layer--pins" viewBox="0 0 {MAP_W} {MAP_H}" role="list" aria-label={$t('battleMapLabel')}>
        <defs>
          <radialGradient id="etude-halo">
            <stop offset="0" stop-color="#e3a15a" stop-opacity="0.55" />
            <stop offset="1" stop-color="#e3a15a" stop-opacity="0" />
          </radialGradient>
        </defs>
        {#each challenges as c, i (c.id)}
          {@const pin = road.pins[i]}
          {@const kind = kindOf(c)}
          {@const lines = labelLines(titleOf(c))}
          {#if pin}
            <g
              class="pin"
              class:pin--current={i === current}
              class:pin--open={openId === c.id}
              class:pin--won={c.marks?.won}
              transform="translate({pin.x.toFixed(1)} {pin.y.toFixed(1)})"
              role="listitem"
            >
              <g
                class="pin-hit"
                role="button"
                tabindex="0"
                aria-label="{roman(i + 1)}. {titleOf(c)}"
                aria-expanded={openId === c.id}
                onclick={() => openPin(c)}
                onkeydown={(e) => onPinKey(e, c)}
                style="--d:{1.5 + i * 0.12}s"
              >
                {#if i === current}
                  <circle class="halo" r="80" cy="-20" fill="url(#etude-halo)" />
                  <!-- Следующий бой отмечен формой, а не только цветом подписи:
                       двойное кольцо на земле видно и тому, кто не различает
                       красный. -->
                  <ellipse class="next-ring" rx="50" ry="14" cy="2" />
                  <ellipse class="next-ring next-ring--outer" rx="62" ry="19" cy="2" />
                {/if}
                <circle class="hit" r="44" cy="-24" />
                <ellipse class="pin-shadow" rx="32" ry="7" cy="2" />
                <g class="glyph-place" transform="scale(1.5)"><g class="glyph">
                  <EtudeGlyph {kind} />
                </g></g>
                {#if c.marks?.won}
                  <g class="seal" class:seal--clean={c.marks.clean} transform="translate(32 -58)">
                    <path
                      class="seal-wax"
                      d="M0 -10l3 1l3 -1l2 3l3 1l0 3l2 3l-2 3l0 3l-3 1l-2 3l-3 -1l-3 1l-3 -1l-3 1l-2 -3l-3 -1l0 -3l-2 -3l2 -3l0 -3l3 -1l2 -3l3 1z"
                    />
                    <circle class="seal-ring" r="5.5" />
                  </g>
                {:else if c.marks?.finished}
                  <circle class="seal-trace" r="9" cx="32" cy="-58" />
                {/if}
                <text class="pin-number" y="-92">{roman(i + 1)}</text>
                <text class="pin-label" y={pin.labelBelow ? 34 : -114 - (lines.length - 1) * 24}>
                  {#each lines as line, l (l)}
                    <tspan x="0" dy={l === 0 ? 0 : 24}>{line}</tspan>
                  {/each}
                </text>
              </g>
            </g>
          {/if}
        {/each}
      </svg>

      <div class="light" aria-hidden="true" bind:this={lightEl}></div>

      <!-- Знаки родов. Не текстом внутри SVG: там кегль уменьшался вместе с
           картой и на 1440 px выходил около десяти точек. Здесь размер
           задан в rem и не зависит от ширины карты. -->
      <ul class="legend">
        <li>
          <svg viewBox="-14 -60 28 64" aria-hidden="true"><EtudeGlyph kind="study" flag={false} /></svg>
          {$t('battleMapKindStudy')}
        </li>
        <li>
          <svg viewBox="-22 -60 44 64" aria-hidden="true"><EtudeGlyph kind="meeting" flag={false} /></svg>
          {$t('battleMapKindMeeting')}
        </li>
        <li>
          <svg viewBox="-31 -60 62 64" aria-hidden="true"><EtudeGlyph kind="city" flag={false} /></svg>
          {$t('battleMapKindCity')}
        </li>
      </ul>
    </div>

    {#if opened}
      <div
        class="leaf"
        style={sheetPlace}
        role="dialog"
        aria-label={titleOf(opened)}
        transition:fade={{ duration: still ? 0 : 180 }}
      >
        <div class="leaf-inner">
          <button type="button" class="leaf-close" aria-label={$t('battleMapClose')} onclick={() => (openId = null)}>×</button>
          <p class="leaf-number">{roman(openedIndex + 1)}</p>
          {@render sheet(opened, false)}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .scroll {
    position: relative;
    width: 100%;
    overflow-x: auto;
    overflow-y: visible;
    scrollbar-width: thin;
    padding: 0 0 1rem;
  }

  .map {
    --ink: #3d2a1e;
    --paper: #f3e7cd;
    position: relative;
    min-width: 780px;
    /* Карта целиком помещается в первый экран: ширина — та, при которой
       высота 16:10 равна окну без того, что стоит над картой
       (`--map-above` задаёт страница). Знак, ушедший под сгиб, — это бой,
       которого не видно. Ниже 780 px карта не сжимается, а прокручивается. */
    width: min(100%, max(780px, calc((100svh - var(--map-above, 0px)) * 1.6)));
    aspect-ratio: 16 / 10;
    margin: 0 auto;
  }

  .sheet {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: 3px;
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.4) inset,
      0 30px 60px -28px rgba(52, 37, 28, 0.55),
      0 10px 24px -12px rgba(52, 37, 28, 0.35);
    transition: transform 0.9s cubic-bezier(0.2, 0.7, 0.2, 1);
    animation: unroll 1.7s cubic-bezier(0.75, 0, 0.2, 1) both;
  }

  @keyframes unroll {
    from {
      clip-path: inset(0 50% 0 50%);
      filter: brightness(0.85);
    }
    to {
      clip-path: inset(0 0 0 0);
      filter: none;
    }
  }

  .picture,
  .layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  /* Картинка проявляется слева направо — один раз. */
  .picture {
    animation: wipe 1.6s cubic-bezier(0.5, 0, 0.2, 1) both;
  }

  @keyframes wipe {
    from {
      clip-path: inset(0 100% 0 0);
      opacity: 0.4;
    }
    to {
      clip-path: inset(0 0 0 0);
      opacity: 1;
    }
  }

  /* Свой слой у движущегося и у знаков: их кадры не трогают остальное. */
  .layer--motion,
  .layer--pins {
    will-change: transform;
  }

  .layer--motion {
    pointer-events: none;
    animation: appear 1.6s 0.6s ease both;
  }

  .paper {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(ellipse at 50% 50%, #f4e8cf 40%, #e2cfaa 100%);
  }

  .layer {
    overflow: visible;
  }

  .layer--pins {
    pointer-events: none;
  }

  /* ── Местность ── */

  .lake {
    animation: appear 1.6s 0.6s ease both;
  }

  .lake-sheen {
    mix-blend-mode: soft-light;
    transform-box: fill-box;
    animation: sheen 16s ease-in-out infinite;
  }

  @keyframes sheen {
    0%,
    100% {
      transform: translateX(-45%);
    }
    50% {
      transform: translateX(45%);
    }
  }

  .lake-hatch {
    fill: none;
    stroke: #3a2c20;
    stroke-width: 0.75;
    opacity: 0.42;
    animation: flow linear infinite;
  }

  .lake-hatch--0 {
    stroke-dasharray: 22 5 9 4;
    animation-duration: 26s;
  }

  .lake-hatch--1 {
    stroke-dasharray: 14 6 30 3;
    animation-duration: 34s;
    animation-direction: reverse;
  }

  .lake-hatch--2 {
    stroke-dasharray: 9 4 17 7;
    animation-duration: 41s;
  }

  @keyframes flow {
    to {
      stroke-dashoffset: -240;
    }
  }

  .lake-ripple {
    fill: none;
    stroke: #3e2c20;
    stroke-width: 0.8;
    stroke-linecap: round;
    animation: lake-ripple 9s var(--d) ease-in-out infinite;
  }

  @keyframes lake-ripple {
    0%,
    100% {
      opacity: 0.1;
      transform: translateX(-4px);
    }
    50% {
      opacity: 0.45;
      transform: translateX(4px);
    }
  }

  .lake-glint {
    stroke: #fffaf0;
    stroke-width: 1.4;
    stroke-linecap: round;
    opacity: 0;
    animation: glint var(--t) var(--d) ease-in-out infinite;
  }

  @keyframes glint {
    0%,
    100% {
      opacity: 0;
      transform: translateX(0);
    }
    45% {
      opacity: 0.95;
    }
    60% {
      opacity: 0.2;
      transform: translateX(5px);
    }
  }

  .reeds {
    transform-box: fill-box;
    transform-origin: 50% 100%;
    animation: reeds 5.5s ease-in-out infinite;
  }

  @keyframes reeds {
    0%,
    100% {
      transform: skewX(0deg);
    }
    50% {
      transform: skewX(-4deg);
    }
  }

  .reed {
    fill: none;
    stroke: #3a2c1f;
    stroke-width: 1;
    stroke-linecap: round;
  }

  .reed-head {
    fill: none;
    stroke: #5a3b24;
    stroke-width: 3;
    stroke-linecap: round;
  }

  .wave {
    fill: none;
    stroke: #4a3a2c;
    stroke-width: 1.1;
    stroke-linecap: round;
    opacity: 0.5;
    animation: swell 6s var(--d) ease-in-out infinite;
  }

  @keyframes swell {
    0%,
    100% {
      transform: translate(0, 0);
      opacity: 0.25;
    }
    50% {
      transform: translate(3px, -2px);
      opacity: 0.6;
    }
  }

  .serpent path {
    fill: none;
    stroke: var(--ink);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .serpent .s-hump,
  .serpent .s-neck {
    fill: #e2d5b4;
  }

  .s-water {
    opacity: 0.6;
  }

  .s-eye {
    fill: var(--ink);
  }

  .s-hump,
  .s-head,
  .s-tail {
    animation: coil 7s ease-in-out infinite;
  }

  .s-hump--2 {
    animation-delay: -1.2s;
  }

  .s-hump--3 {
    animation-delay: -2.4s;
  }

  .s-tail {
    animation-delay: -3.4s;
  }

  @keyframes coil {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(4px);
    }
  }

  /* ── Дорога ── */

  .road {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .road--bed {
    stroke: rgba(246, 236, 212, 0.55);
    stroke-width: 9;
    animation: appear 1.2s 1.1s ease both;
  }

  .road--ahead {
    stroke: #6f3b24;
    stroke-width: 3;
    stroke-dasharray: 0.5 9;
    opacity: 0.85;
    animation: appear 1.2s 1.1s ease both;
  }

  .road--walked {
    stroke: #8f3a22;
    stroke-width: 4;
    stroke-dasharray: 12 7;
  }

  .walk {
    stroke-dasharray: var(--walk) 2;
    animation: walk 2.8s 1.4s cubic-bezier(0.45, 0, 0.2, 1) both;
  }

  @keyframes walk {
    from {
      stroke-dashoffset: var(--walk);
    }
    to {
      stroke-dashoffset: 0;
    }
  }

  @keyframes appear {
    from {
      opacity: 0;
    }
  }

  /* ── Корабль и птицы ── */

  .ship path {
    stroke: var(--ink);
    stroke-width: 1.2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .ship-hull {
    fill: #6f3b24;
  }

  .ship-sail {
    fill: #f6ecd4;
  }

  .ship-mast,
  .ship-wake {
    fill: none;
  }

  .ship-wake {
    opacity: 0.5;
  }

  .ship-pennant {
    fill: #a8442a;
  }

  .ship-bob {
    animation: bob 4.5s ease-in-out infinite;
  }

  @keyframes bob {
    0%,
    100% {
      transform: rotate(-3deg) translateY(0);
    }
    50% {
      transform: rotate(3deg) translateY(1.5px);
    }
  }

  .bird {
    fill: none;
    stroke: #34251c;
    stroke-width: 1.3;
    stroke-linecap: round;
    stroke-linejoin: round;
    transform-box: fill-box;
    transform-origin: 50% 100%;
    animation: flap 0.9s var(--d) ease-in-out infinite;
    opacity: 0.75;
  }

  @keyframes flap {
    50% {
      transform: scaleY(0.25);
    }
  }

  /* ── Роза ветров ── */

  .c-ring {
    fill: rgba(243, 231, 205, 0.55);
    stroke: var(--ink);
    stroke-width: 1.4;
  }

  .c-ring--thin {
    fill: none;
    stroke-width: 0.7;
  }

  .c-ticks {
    fill: none;
    stroke: var(--ink);
    stroke-width: 6;
    stroke-dasharray: 1 10;
    opacity: 0.6;
  }

  .c-needle {
    animation: sway 11s ease-in-out infinite;
  }

  @keyframes sway {
    0%,
    100% {
      transform: rotate(-3deg);
    }
    50% {
      transform: rotate(3deg);
    }
  }

  .c-point {
    fill: #f6ecd4;
    stroke: var(--ink);
    stroke-width: 0.9;
    stroke-linejoin: round;
  }

  .c-point--minor {
    fill: #efe1c2;
  }

  .c-point-dark {
    fill: #6f3b24;
    stroke: var(--ink);
    stroke-width: 0.9;
    stroke-linejoin: round;
  }

  .c-hub {
    fill: #a8442a;
    stroke: var(--ink);
  }

  .c-north {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 26px;
    font-weight: 600;
    text-anchor: middle;
    fill: var(--ink);
  }

  /* ── Картуш, легенда, рамка ── */

  .cartouche {
    animation: appear 1.4s 0.9s ease both;
  }

  .ct-scroll {
    fill: rgba(246, 236, 212, 0.92);
    stroke: var(--ink);
    stroke-width: 1.6;
  }

  .ct-scroll-inner {
    fill: none;
    stroke: #6f3b24;
    stroke-width: 0.8;
  }

  .ct-curl,
  .ct-rule {
    fill: none;
    stroke: var(--ink);
    stroke-width: 1.3;
    stroke-linecap: round;
  }

  .ct-rule {
    fill: #a8442a;
  }

  .ct-title {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 40px;
    font-style: italic;
    font-weight: 500;
    text-anchor: middle;
    fill: var(--ink);
  }

  .legend {
    position: absolute;
    left: 2.6%;
    bottom: 3.2%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem 1.1rem;
    margin: 0;
    padding: 0.35rem 0.8rem;
    list-style: none;
    background: rgba(246, 236, 212, 0.9);
    border: 1px solid var(--ink);
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 1.05rem;
    font-style: italic;
    line-height: 1.2;
    color: var(--ink);
    pointer-events: none;
    animation: appear 1.4s 1.2s ease both;
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .legend svg {
    height: 1.35rem;
    width: auto;
    overflow: visible;
  }

  .frame {
    fill: none;
    stroke: var(--ink);
  }

  .frame--outer {
    stroke-width: 2;
  }

  .frame--band {
    stroke-width: 6;
    stroke-dasharray: 50 50;
    opacity: 0.7;
  }

  .frame--inner {
    stroke-width: 1;
  }

  /* ── Облака ── */

  .clouds {
    position: absolute;
    inset: 0;
    pointer-events: none;
    transition: transform 1.4s cubic-bezier(0.2, 0.7, 0.2, 1);
    animation: appear 3s ease both;
  }

  .drift {
    position: absolute;
    inset: 0;
    will-change: transform;
    animation: sail var(--dur) linear infinite;
    animation-delay: var(--delay);
  }

  .drift--0 {
    --dur: 150s;
    --delay: -40s;
    --top: 18%;
    --w: 34%;
  }

  .drift--1 {
    --dur: 190s;
    --delay: -130s;
    --top: 55%;
    --w: 40%;
  }

  .drift--2 {
    --dur: 170s;
    --delay: -95s;
    --top: 74%;
    --w: 28%;
  }

  @keyframes sail {
    from {
      transform: translateX(-45%);
    }
    to {
      transform: translateX(105%);
    }
  }

  .cloud,
  .cloud-shadow {
    position: absolute;
    left: 0;
    top: var(--top);
    width: var(--w);
    height: auto;
  }

  .cloud {
    opacity: 0.62;
  }

  .cloud-shadow {
    filter: brightness(0.15) blur(5px);
    opacity: 0.13;
    transform: translate(2.2%, 46%);
  }

  /* ── Свет ── */

  /* Тёплый свет за указателем: пятно двигается сдвигом, а не перекраской
     фона, — такое движение браузер отдаёт видеокарте. */
  .light {
    position: absolute;
    left: -30%;
    top: -48%;
    width: 60%;
    aspect-ratio: 1;
    pointer-events: none;
    background: radial-gradient(
      circle,
      rgba(255, 220, 160, 0.16) 0,
      rgba(255, 220, 160, 0.05) 45%,
      rgba(255, 220, 160, 0) 70%
    );
    mix-blend-mode: soft-light;
    opacity: 0;
    will-change: transform;
    transition: opacity 1.2s ease;
  }


  /* ── Знаки этюдов ── */

  .pin-hit {
    pointer-events: auto;
    cursor: pointer;
    outline: none;
    animation: land 0.8s var(--d) cubic-bezier(0.3, 1.5, 0.5, 1) both;
  }

  @keyframes land {
    from {
      opacity: 0;
      transform: translateY(-40px) scale(0.6);
    }
    55% {
      opacity: 1;
    }
  }

  .hit {
    fill: transparent;
  }

  .pin-shadow {
    fill: rgba(52, 37, 28, 0.28);
  }

  .glyph {
    transition: transform 0.35s cubic-bezier(0.3, 1.6, 0.5, 1);
  }

  .pin-hit:hover .glyph,
  .pin-hit:focus-visible .glyph,
  .pin--open .glyph {
    transform: translateY(-5px) scale(1.08);
  }

  .next-ring {
    fill: none;
    stroke: #6f3b24;
    stroke-width: 2.2;
  }

  .next-ring--outer {
    stroke-width: 1.2;
    stroke-dasharray: 5 4;
  }

  .halo {
    animation: breathe 4.5s ease-in-out infinite;
  }

  @keyframes breathe {
    0%,
    100% {
      opacity: 0.55;
    }
    50% {
      opacity: 1;
    }
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

  .seal--clean .seal-ring {
    stroke: #d9b45a;
    stroke-width: 1.8;
  }

  .seal-trace {
    fill: none;
    stroke: #6f3b24;
    stroke-width: 1.2;
    stroke-dasharray: 2 2.4;
  }

  .pin-number {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-anchor: middle;
    fill: #6f3b24;
    paint-order: stroke;
    stroke: rgba(243, 231, 205, 0.9);
    stroke-width: 4px;
  }

  .pin-label {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 25px;
    font-style: italic;
    font-weight: 600;
    text-anchor: middle;
    fill: var(--ink);
    paint-order: stroke;
    stroke: rgba(243, 231, 205, 0.92);
    stroke-width: 5px;
    stroke-linejoin: round;
    transition: fill 0.25s ease;
  }

  .pin--current .pin-label,
  .pin-hit:hover .pin-label,
  .pin-hit:focus-visible .pin-label,
  .pin--open .pin-label {
    fill: #a8442a;
  }

  .pin-hit:focus-visible .hit {
    stroke: #c65f3c;
    stroke-width: 1.5;
    stroke-dasharray: 3 3;
  }

  /* ── Листок этюда ── */

  .leaf {
    position: absolute;
    z-index: 5;
    width: min(22rem, 80vw);
    transform: translate(var(--sx), var(--sy));
  }

  .leaf-inner {
    position: relative;
    padding: 1.3rem 1.4rem 1.2rem;
    background: #f8f1e7;
    border: 1px solid #b89b7c;
    outline: 1px solid #d8c6b1;
    outline-offset: 4px;
    box-shadow: 0 24px 50px -20px rgba(52, 37, 28, 0.6);
    transform: rotate(-1deg);
    color: #34251c;
  }

  .leaf-number {
    margin: 0 0 0.2rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 0.85rem;
    letter-spacing: 0.2em;
    color: #6f3b24;
  }

  .leaf-close {
    position: absolute;
    top: 0.35rem;
    right: 0.55rem;
    padding: 0.1rem 0.35rem;
    font-size: 1.3rem;
    line-height: 1;
    color: #6f3b24;
    background: none;
    border: 0;
    cursor: pointer;
  }

  .leaf-close:hover {
    color: #c65f3c;
  }

  /* Уже 760 px карты нет: страница ставит вместо неё дорогу (`EtudeRoad`). */

  /* Без движения карта стоит готовой. */
  .map--still *,
  .map--still {
    animation: none !important;
    transition: none !important;
  }

  .map--still .sheet {
    transform: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .map *,
    .map {
      animation: none !important;
      transition: none !important;
    }
  }

  .drawing {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
    font-size: 1rem;
    color: #6f3b24;
    opacity: 0.7;
    pointer-events: none;
  }
</style>
