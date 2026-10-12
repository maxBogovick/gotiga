<script lang="ts">
  // Плита на полях к байке: фигурка работы и одно движение, которое растёт
  // вместе с чтением. Девять байк — девять движений, но устройство одно.
  //
  // Правило отбора такое же, как у сцены про перегрев (`TaleStageHeat`):
  // сцена показывает то, чего в тексте нет, а не пересказывает его. Поэтому
  // слов на плите нет, а у каждой байки движение своё и про её смысл:
  //
  //   frames   — кадры одной работы, и между каждой парой свой переход
  //              (наплыв, наезд, свет, рывок, подъём, прилив, крещендо) в
  //              момент, который назначил автор или подобрал алгоритм по тексту
  //              (`sceneTiming.ts`); правится на столе рассказов;
  //   dusk     — «Пока время не стало взрослее»: день уходит, тень растёт;
  //   print    — «Пирожки всему могут научить»: за спиной газетная полоса;
  //   ruler    — «Отмеряльщик от бога»: разматывается мерная лента;
  //   rings    — «Лечу по криптографии…»: от бубна расходятся круги;
  //   fog      — «Лесник и его невспоминаемые приключения»: фигура уходит в туман.
  //
  // Устройство. Всё считается из **одного числа** `--grow` (0 … 1), которое
  // страница или сама плита двигает по прокрутке (`animation-timeline`):
  // остальное — формулы CSS (`calc`, `clamp`, `min`) над ним. Второго числа
  // нет, поэтому движения не могут разойтись друг с другом, а плита без
  // поддержки шкалы стоит в одном определённом положении (`initial-value`), а
  // не в нулевом. Это же положение показывается при
  // `prefers-reduced-motion`.
  //
  // Шкалу даёт страница (`--scene-timeline`, `--scene-range`): жар растёт по
  // ходу чтения всей байки, а не по тому, где плита стоит на экране. Без неё
  // плита меряет сама себя, как на узком экране, где колонка падает в текст.
  //
  // Время в плите есть только у того, что дышит само (качание фигуры, туман).
  // Оно стоит, пока плиты нет на экране (`live`).
  import type { PlateEffect } from '$lib/taleScenes';

  let {
    figure,
    effect,
    bleed = false,
    frames = [],
    transitions = [],
    ratio = '3 / 4',
    position = '50% 50%',
    grow = undefined,
  }: {
    /** Вырезанная фигурка, WebP с прозрачностью. */
    figure: string;
    effect: PlateEffect;
    /**
     * Фигура обрезана по краям кадра (бюст): она уходит за низ и боковые
     * края плиты, а не стоит на полу.
     */
    bleed?: boolean;
    /**
     * Для `frames`: кадры по порядку. Это целые фотографии вместе с фоном, а не
     * вырезки: у кадров одной работы один стол и одна стена, и вырезать их
     * значило бы заново рисовать то, что на снимках уже сходится. У кадра может
     * быть своё `position`, если фигура стоит в нём не там, где в остальных.
     */
    frames?: { url: string; position?: string }[];
    /**
     * Для `frames`: переход `i` ведёт из кадра `i` в кадр `i + 1`. `at` — где он
     * идёт долей чтения, `span` — сколько длится.
     */
    transitions?: { type: string; at: number; span: number }[];
    /**
     * Пропорции плиты, `ширина / высота`. По умолчанию 3 / 4; у байки с
     * высокими снимками (1 / 3,4) плита вытянута до 1 / 2, а окно поставлено
     * на фигуру.
     */
    ratio?: string;
    /** Для кадров: `object-position`, если фигура стоит не по центру. */
    position?: string;
    /**
     * Показать плиту на этом числе (0 … 1) и остановить: так стол рассказов
     * проматывает сцену ползунком, не двигая страницу. Без него число ведёт
     * прокрутка.
     */
    grow?: number;
  } = $props();

  let hallStyle = $derived(
    `--plate-ratio: ${ratio}; --frame-pos: ${position}` + (grow === undefined ? '' : `; --grow: ${grow}; animation: none`),
  );

  // Окно перехода: он стоит вокруг своего `at` и не вылезает за чтение.
  // Без переходов (сцена от кода) кадры расходятся поровну и плавно.
  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
  let trs = $derived(
    transitions.length === frames.length - 1
      ? transitions
      : frames.slice(1).map((_, k) => ({ type: 'fade', at: (k + 1) / frames.length, span: 0.9 / Math.max(1, frames.length - 1) })),
  );
  function win(k: number): { s: number; w: number } {
    const w = clamp(trs[k]?.span ?? 0.14, 0.04, 0.6);
    return { s: clamp((trs[k]?.at ?? 0.5) - w / 2, 0, 1 - w), w };
  }
  const num = (v: number) => +v.toFixed(4);
  const incoming = (i: number) => (i === 0 ? '' : `--s: ${num(win(i - 1).s)}; --w: ${num(win(i - 1).w)}`);
  const outgoing = (i: number) => (i >= trs.length ? '' : `--ns: ${num(win(i).s)}; --nw: ${num(win(i).w)}`);

  let live = $state(false);

  function watch(node: HTMLElement) {
    if (typeof IntersectionObserver === 'undefined') {
      live = true;
      return;
    }
    const watcher = new IntersectionObserver(
      (entries) => {
        live = entries.some((e) => e.isIntersecting);
      },
      { threshold: 0.12 },
    );
    watcher.observe(node);
    return { destroy: () => watcher.disconnect() };
  }

  // Круги от бубна: девять, и каждый встаёт, когда `--grow` дойдёт до его
  // номера.
  const RINGS = Array.from({ length: 9 }, (_, i) => 7 + i * 6.4);

  // Мерная лента: деления считаются, а не рисуются руками.
  const NUMERALS = Array.from({ length: 9 }, (_, i) => (i + 1) * 10);
</script>

<figure class="plate" class:live class:bleed data-effect={effect} use:watch aria-hidden="true">
  <div class="hall" style={hallStyle}>
    <div class="wall"></div>

    {#if effect === 'dusk'}
      <div class="sky sky--dusk"></div>
      <div class="sky sky--day"></div>
      <div class="horizon"></div>
    {/if}

    {#if effect === 'fog'}
      <div class="snow"></div>
    {/if}

    {#if effect === 'print'}
      <div class="news">
        <div class="news-cols"></div>
        <i style="top: 6%; width: 62%"></i>
        <i style="top: 26%; width: 44%"></i>
        <i style="top: 47%; width: 70%"></i>
        <i style="top: 69%; width: 52%"></i>
        <i style="top: 88%; width: 66%"></i>
      </div>
    {/if}

    {#if effect === 'rings'}
      <svg class="rings" viewBox="0 0 100 133" focusable="false">
        {#each RINGS as r, i}
          <circle class="ring" style="--i: {i}" cx="50" cy="72" {r} />
        {/each}
      </svg>
    {/if}

    {#if effect === 'ruler'}
      <div class="tape">
        <div class="tape-roll">
          {#each NUMERALS as n, i}
            <span style="top: {(i + 1) * 10}%">{n}</span>
          {/each}
          <div class="caliper"></div>
        </div>
      </div>
    {/if}

    {#if effect === 'frames'}
      <div class="frames">
        {#each frames as frame, i}
          <div class="fr" data-tr={i === 0 ? undefined : trs[i - 1]?.type} style={incoming(i)}>
            <img
              class="fi"
              data-ntr={trs[i]?.type}
              style="{outgoing(i)}{frame.position ? `; object-position: ${frame.position}` : ''}"
              src={frame.url}
              alt=""
              loading="lazy"
              decoding="async"
            />
          </div>
        {/each}
        {#each trs as t, k}
          <div class="fx" data-tr={t.type} style={incoming(k + 1)}></div>
        {/each}
      </div>
    {:else}
      <div class="ground"></div>

      <div class="fig-box">
        <div class="sway">
          <img class="fig" src={figure} alt="" loading="lazy" decoding="async" />
        </div>
      </div>

    {/if}

    {#if effect === 'fog'}
      <div class="mist mist--a"></div>
      <div class="mist mist--b"></div>
    {/if}

    {#if effect !== 'frames'}
      <div class="floor"></div>
    {/if}
    <div class="grain"></div>
    <div class="shade"></div>
  </div>
</figure>

<style>
  /* Единственное число плиты. Начальное значение — то положение, в котором
     плита стоит без шкалы прокрутки и при `prefers-reduced-motion`. */
  @property --grow {
    syntax: '<number>';
    inherits: true;
    initial-value: 0.85;
  }

  .plate {
    margin: 0;
    position: relative;
    view-timeline-name: --plate;
    view-timeline-axis: block;
    border: 1px solid #d8c6b1;
    padding: 5px;
    background: #f8f1e7;
    box-shadow: 0 18px 38px -22px rgba(52, 37, 28, 0.55);
    --run: paused;
  }

  .plate.live {
    --run: running;
  }

  .hall {
    position: relative;
    overflow: hidden;
    aspect-ratio: var(--plate-ratio, 3 / 4);
    max-height: 80vh;
    background: #0b0908;
    isolation: isolate;
    container-type: size;
    border: 1px solid #6f3b24;
  }

  .hall > * {
    position: absolute;
    inset: 0;
  }

  /* ── стена: у каждой байки свой тон ──────────────────────────────── */

  .wall {
    background: radial-gradient(110% 90% at 40% 62%, var(--wa, #2a241f) 0%, var(--wb, #15110e) 56%, #0a0807 100%);
  }

  [data-effect='dusk'] { --wa: #2a2420; --wb: #12100e; }
  [data-effect='print'] { --wa: #2d241a; --wb: #15100b; }
  [data-effect='ruler'] { --wa: #2a261c; --wb: #14120d; }
  [data-effect='rings'] { --wa: #22202e; --wb: #100f17; }
  [data-effect='fog'] { --wa: #4a3318; --wb: #1f160b; }

  /* ── фигурка ─────────────────────────────────────────────────────── */

  .fig-box {
    inset: 12cqh 6cqw 10cqh 6cqw;
  }

  .bleed .fig-box {
    inset: 10cqh 0 -2cqh 0;
  }

  .sway {
    width: 100%;
    height: 100%;
    transform-origin: 50% 100%;
    animation: sway 7s ease-in-out infinite;
    animation-play-state: var(--run);
  }

  .fig {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    object-position: 50% 100%;
    filter: drop-shadow(0 6px 10px rgba(0, 0, 0, 0.55));
  }

  .ground {
    inset: auto 14cqw 9cqh 14cqw;
    height: 4cqh;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgba(0, 0, 0, 0.6), transparent);
    filter: blur(2px);
  }

  .bleed .ground { display: none; }

  .floor {
    inset: auto 0 0 0;
    height: 22%;
    background: linear-gradient(180deg, transparent, #070504 90%);
    pointer-events: none;
  }

  .grain {
    pointer-events: none;
    mix-blend-mode: overlay;
    opacity: 0.2;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='g'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 1.1 -0.2'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23g)'/%3E%3C/svg%3E");
  }

  .shade {
    pointer-events: none;
    background: radial-gradient(90% 90% at 50% 48%, transparent 52%, rgba(5, 3, 2, 0.72) 100%);
  }

  /* ── frames: кадры и переходы между ними ─────────────────────────── */

  /* Каждый кадр — две коробки. Внешняя (`.fr`) несёт то, как кадр ПРИХОДИТ:
     прозрачность, маску, сдвиг; внутренняя (`.fi`) — то, как он УХОДИТ, когда
     приходит следующий: приближение и смаз. Так приход и уход живут в разных
     коробках, и у них могут быть разные точки, от которых кадр растёт. Один
     `transform` на двоих эти точки перепутал бы.

     Тип перехода — значение `data-tr` у приходящего кадра, `data-ntr` у
     уходящего; всё, что движется, считается из общего `--grow` и окна
     перехода (`--s`, `--w`). Первый кадр стоит всегда. */
  .frames {
    inset: 0;
    transform-origin: 52% 36%;
    transform: scale(calc(1 + var(--grow) * 0.02));
  }

  .fr {
    position: absolute;
    inset: 0;
    --x: clamp(0, (var(--grow) - var(--s, 0)) / var(--w, 1), 1);
    --e: calc(var(--x) * var(--x) * (3 - 2 * var(--x)));
  }

  .fi {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: var(--frame-pos, 50% 50%);
    --nx: clamp(0, (var(--grow) - var(--ns, 2)) / var(--nw, 1), 1);
  }

  /* Приход. Наплыв — основа: следующий кадр проступает, и ничего больше. */
  .fr[data-tr] {
    opacity: var(--e);
  }

  /* Наезд: кадр подъезжает из увеличения. */
  .fr[data-tr='push'] {
    transform-origin: 50% 70%;
    scale: calc(1.2 - var(--x) * 0.2);
  }

  /* Свет: кадр вырастает кругом из точки у низа, и виден только внутри круга. */
  .fr[data-tr='bloom'] {
    opacity: 1;
    -webkit-mask-image: radial-gradient(
      circle at 50% 74%,
      #000 calc(var(--x) * 135% - 30%),
      transparent calc(var(--x) * 135%)
    );
    mask-image: radial-gradient(
      circle at 50% 74%,
      #000 calc(var(--x) * 135% - 30%),
      transparent calc(var(--x) * 135%)
    );
    transform-origin: 50% 70%;
    scale: calc(1.1 - var(--x) * 0.1);
  }

  /* Рывок: кадр въезжает сбоку, граница мягкая, края смазаны. */
  .fr[data-tr='wipe'] {
    opacity: 1;
    -webkit-mask-image: linear-gradient(270deg, #000 calc(var(--x) * 140% - 40%), transparent calc(var(--x) * 140%));
    mask-image: linear-gradient(270deg, #000 calc(var(--x) * 140% - 40%), transparent calc(var(--x) * 140%));
    translate: calc((1 - var(--x)) * 7%) 0;
    filter: blur(calc((1 - var(--x)) * 4px));
  }

  /* Подъём: камера уходит вверх, кадр открывается сверху вниз. */
  .fr[data-tr='rise'] {
    opacity: 1;
    -webkit-mask-image: linear-gradient(180deg, #000 calc(var(--x) * 140% - 40%), transparent calc(var(--x) * 140%));
    mask-image: linear-gradient(180deg, #000 calc(var(--x) * 140% - 40%), transparent calc(var(--x) * 140%));
    translate: 0 calc((var(--x) - 1) * 12%);
    scale: calc(1.14 - var(--x) * 0.14);
    transform-origin: 50% 20%;
  }

  /* Крещендо: кадр бросается из большого масштаба и встаёт на место. */
  .fr[data-tr='surge'] {
    opacity: var(--x);
    transform-origin: 50% 46%;
    scale: calc(1.4 - var(--x) * 0.4);
  }

  /* Уход. */
  .fi[data-ntr='push'] {
    transform-origin: 56% 66%;
    scale: calc(1 + var(--nx) * 0.4);
  }

  .fi[data-ntr='flush'] {
    transform-origin: 60% 56%;
    scale: calc(1 + var(--nx) * 0.16);
  }

  .fi[data-ntr='wipe'] {
    filter: blur(calc(var(--nx) * 6px));
  }

  .fi[data-ntr='surge'] {
    transform-origin: 50% 46%;
    scale: calc(1 + var(--nx) * 0.4);
  }

  /* Свет поверх перехода: у света, прилива и крещендо есть своя подкладка. */
  .fx {
    position: absolute;
    inset: 0;
    pointer-events: none;
    --x: clamp(0, (var(--grow) - var(--s, 0)) / var(--w, 1), 1);
    --tent: calc(min(var(--x), 1 - var(--x)));
  }

  .fx[data-tr='fade'],
  .fx[data-tr='push'],
  .fx[data-tr='wipe'],
  .fx[data-tr='rise'] {
    display: none;
  }

  /* Тёплое кольцо на кромке круга: свет лежит на краю, а не заливает кадр. */
  .fx[data-tr='bloom'] {
    mix-blend-mode: soft-light;
    background: radial-gradient(
      circle at 50% 74%,
      transparent calc(var(--x) * 135% - 24%),
      rgba(255, 206, 140, 1) calc(var(--x) * 135% - 10%),
      transparent calc(var(--x) * 135% + 3%)
    );
    opacity: min(1, calc(var(--tent) * 3));
  }

  /* Прилив крови: на время перехода лицо заливает красным. */
  .fx[data-tr='flush'] {
    mix-blend-mode: soft-light;
    background: radial-gradient(70% 55% at 62% 50%, #e03a22 0%, #8a1208 80%);
    opacity: min(1, calc(var(--tent) * 2.4));
  }

  /* Крещендо: на миг кадр горячее. */
  .fx[data-tr='surge'] {
    mix-blend-mode: screen;
    background: rgba(255, 168, 98, 0.3);
    opacity: min(1, calc(var(--tent) * 2.6));
  }

  /* ── dusk: день уходит, тень растёт ──────────────────────────────── */

  .sky {
    inset: 0 0 38% 0;
  }

  .sky--dusk {
    background: linear-gradient(180deg, #161a27 0%, #3a2c33 55%, #8a4e30 100%);
  }

  .sky--day {
    background: linear-gradient(180deg, #8d9496 0%, #b9b79f 60%, #d8c394 100%);
    opacity: calc(1 - var(--grow));
  }

  .horizon {
    inset: auto 0 28% 0;
    height: 24%;
    background: radial-gradient(60% 100% at 78% 100%, rgba(255, 150, 80, 0.7), transparent 70%);
    opacity: calc(var(--grow) * 0.9);
    mix-blend-mode: screen;
  }

  [data-effect='dusk'] .ground {
    inset: auto 8cqw 9cqh 10cqw;
    height: 5cqh;
    transform-origin: 20% 50%;
    transform: scaleX(calc(0.4 + var(--grow) * 1.5));
    background: radial-gradient(closest-side, rgba(0, 0, 0, 0.72), transparent);
  }

  [data-effect='dusk'] .fig {
    filter: drop-shadow(0 6px 10px rgba(0, 0, 0, 0.55)) brightness(calc(1 - var(--grow) * 0.38))
      sepia(calc(var(--grow) * 0.3));
  }

  /* ── print: за спиной газетная полоса ────────────────────────────── */

  .news {
    inset: -10% 0 -10% 0;
    opacity: calc(0.14 + var(--grow) * 0.3);
    transform: translateY(calc(var(--grow) * -14%));
    filter: sepia(0.55);
  }

  .news-cols {
    position: absolute;
    inset: 0 6% 0 6%;
    background:
      repeating-linear-gradient(180deg, rgba(222, 205, 168, 0.8) 0 1px, transparent 1px 5px) 0 0 / 30% 100% no-repeat,
      repeating-linear-gradient(180deg, rgba(222, 205, 168, 0.8) 0 1px, transparent 1px 5px) 35% 0 / 30% 100% no-repeat,
      repeating-linear-gradient(180deg, rgba(222, 205, 168, 0.8) 0 1px, transparent 1px 5px) 70% 0 / 30% 100% no-repeat;
  }

  .news i {
    position: absolute;
    left: 6%;
    height: 3.4cqw;
    background: rgba(222, 205, 168, 0.9);
  }

  [data-effect='print'] .fig {
    filter: drop-shadow(0 6px 10px rgba(0, 0, 0, 0.55)) sepia(calc(var(--grow) * 0.3));
  }

  /* ── ruler: разматывается мерная лента ───────────────────────────── */

  .tape {
    inset: 6cqh 5cqw 6cqh auto;
    width: 11cqw;
  }

  .tape-roll {
    position: relative;
    width: 100%;
    height: 100%;
    background:
      repeating-linear-gradient(180deg, #3a2e1c 0 1px, transparent 1px 2cqh) right 0 top 0 / 34% 100% no-repeat,
      repeating-linear-gradient(180deg, #3a2e1c 0 1.5px, transparent 1.5px 10cqh) right 0 top 0 / 62% 100% no-repeat,
      #e6d9ba;
    clip-path: inset(0 0 calc((1 - var(--grow)) * 100%) 0);
    box-shadow: -3px 0 8px rgba(0, 0, 0, 0.4);
  }

  .tape-roll span {
    position: absolute;
    left: 14%;
    transform: translateY(-50%);
    font: italic 600 2.6cqw/1 Georgia, 'Times New Roman', serif;
    color: #3a2e1c;
  }

  .caliper {
    position: absolute;
    right: 100%;
    top: calc(var(--grow) * 100%);
    width: 40cqw;
    height: 0;
    border-top: 1px solid rgba(180, 60, 40, 0.9);
  }

  /* ── rings: круги от бубна ───────────────────────────────────────── */

  .rings {
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .ring {
    fill: none;
    stroke: #cfc6e8;
    stroke-width: 0.3;
    transform-box: fill-box;
    transform-origin: center;
    opacity: clamp(0, var(--grow) * 10 - var(--i), 1);
    transform: scale(calc(0.8 + clamp(0, var(--grow) * 10 - var(--i), 1) * 0.2));
  }

  /* ── fog: фигура уходит в туман ──────────────────────────────────── */

  .snow {
    background: radial-gradient(110% 90% at 40% 62%, #8d8f92 0%, #5c5e61 56%, #2a2b2d 100%);
    opacity: var(--grow);
  }

  [data-effect='fog'] .fig {
    opacity: calc(1 - var(--grow) * 0.6);
    filter: drop-shadow(0 6px 10px rgba(0, 0, 0, 0.55)) blur(calc(var(--grow) * 2px))
      saturate(calc(1 - var(--grow) * 0.45));
  }

  .mist {
    pointer-events: none;
    inset: auto -20% 4% -20%;
    height: 56%;
    background: radial-gradient(closest-side, rgba(236, 238, 240, 0.8), transparent);
    filter: blur(10px);
    opacity: calc(var(--grow) * 0.7);
    animation: drift 18s ease-in-out infinite alternate;
    animation-play-state: var(--run);
  }

  .mist--b {
    inset: auto -30% 22% -10%;
    height: 44%;
    animation-duration: 25s;
    animation-direction: alternate-reverse;
    opacity: calc(var(--grow) * 0.55);
  }

  /* ── ход ─────────────────────────────────────────────────────────── */

  @keyframes sway {
    0%, 100% { transform: rotate(-0.4deg); }
    50% { transform: rotate(0.4deg); }
  }

  @keyframes drift {
    from { transform: translateX(-6%); }
    to { transform: translateX(6%); }
  }

  /* ── от прокрутки ────────────────────────────────────────────────── */

  @supports (animation-timeline: view()) {
    @keyframes grow {
      from { --grow: 0; }
      to { --grow: 1; }
    }

    .hall {
      animation: grow linear both;
      animation-timeline: var(--scene-timeline, --plate);
      animation-range: var(--scene-range, entry 25% cover 65%);
    }

    /* У кадров моменты назначены долями чтения от 0 до 1, поэтому им нужно всё
       чтение, а не его середина. */
    [data-effect='frames'] .hall {
      animation-range: var(--scene-range-frames, var(--scene-range, entry 25% cover 65%));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .hall,
    .sway,
    .mist {
      animation: none;
    }
  }
</style>
