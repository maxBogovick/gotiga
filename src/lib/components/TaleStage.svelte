<script lang="ts">
  // Сцена внутри байки: вырезанная фигурка работы стоит слоями перед окном и
  // свечой, а прокрутка двигает слои, как в театре теней.
  //
  // Рисунка нет: актёр — фотография самой работы (`tools/cutout`), а
  // «багровый свет» трещин — та же фотография, у которой оставлены только
  // красные жилы (`veins`). Поэтому сцена не может разойтись с работой.
  //
  // Движение двух родов, и они не мешают друг другу:
  //  · от прокрутки — фигура поднимается из-за стола, а к концу делает шаг к
  //    читателю (`animation-timeline`, только CSS, шкала объявлена на сцене, а
  //    двигаются дети: сдвиг самого предмета шкалы двигал бы шкалу);
  //  · по времени — стук сердца и свеча. Сердце начинает биться, когда сцена
  //    легла на экран больше чем наполовину (`awake`): биться раньше значило
  //    бы, что никто не видел, как оно началось.
  //
  // Сцена не несёт слов: текст рядом уже рассказал, что происходит, и подпись
  // под картинкой пересказала бы его второй раз. Для чтения с экрана она
  // скрыта целиком. Под `prefers-reduced-motion` всё стоит в конечном виде.

  let {
    figure,
    veins,
  }: {
    /** Вырезанная фигурка, WebP с прозрачностью. */
    figure: string;
    /** Только красные жилы той же фотографии, того же размера. */
    veins: string;
  } = $props();

  let awake = $state(false);

  function wake(node: HTMLElement) {
    if (typeof IntersectionObserver === 'undefined') {
      awake = true;
      return;
    }
    const watcher = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          awake = true;
          watcher.disconnect();
        }
      },
      { threshold: 0.55 },
    );
    watcher.observe(node);
    return { destroy: () => watcher.disconnect() };
  }

  // Шестерня за окном. Города Забытых Механизмов в тексте нет на картинке
  // нигде, кроме этого: поэтому она одна, большая и медленная.
  function gearPath(teeth: number, inner: number, outer: number): string {
    const step = (Math.PI * 2) / teeth;
    const pts: string[] = [];
    for (let i = 0; i < teeth; i++) {
      const a = i * step;
      for (const [da, r] of [
        [-0.3, inner],
        [-0.16, outer],
        [0.16, outer],
        [0.3, inner],
      ] as const) {
        pts.push(`${(Math.cos(a + da * step) * r).toFixed(2)},${(Math.sin(a + da * step) * r).toFixed(2)}`);
      }
    }
    const hole = inner * 0.66;
    return `M${pts.join('L')}Z M${hole},0 A${hole},${hole} 0 1 0 ${-hole},0 A${hole},${hole} 0 1 0 ${hole},0Z`;
  }
  const GEAR = gearPath(14, 78, 96);
  const SPOKES = [0, 60, 120];

  // Пылинки в луче: место и ход назначены раз и навсегда, а не случайны —
  // сцена пререндерится, и случайное число при сборке и при монтировании дало
  // бы два разных расположения.
  const MOTES = Array.from({ length: 16 }, (_, i) => ({
    x: (i * 37 + 11) % 97,
    size: 1.5 + ((i * 13) % 5) * 0.55,
    dur: 16 + ((i * 7) % 11),
    delay: -((i * 5) % 17),
    drift: ((i * 19) % 15) - 7,
  }));
</script>

<figure class="tale-stage" class:awake use:wake aria-hidden="true">
  <div class="room">
    <div class="wall"></div>

    <div class="far">
      <div class="window">
        <div class="pane">
          <svg class="gear" viewBox="-100 -100 200 200" focusable="false">
            <path d={GEAR} fill-rule="evenodd" />
            <circle r="15" />
            {#each SPOKES as deg}
              <rect x="-4" y="-66" width="8" height="132" transform="rotate({deg})" />
            {/each}
          </svg>
          <div class="roofs"></div>
          <div class="rain rain--far"></div>
          <div class="rain rain--near"></div>
        </div>
      </div>
    </div>

    <div class="glow-back"></div>

    <div class="rise">
      <div class="step">
        <div class="beat">
          <img class="fig" src={figure} alt="" width="1100" height="1054" loading="lazy" decoding="async" />
          <div class="veins-wrap">
            <img class="veins veins--bloom" src={veins} alt="" width="1100" height="1054" loading="lazy" decoding="async" />
            <img class="veins veins--line" src={veins} alt="" width="1100" height="1054" loading="lazy" decoding="async" />
          </div>
        </div>
      </div>
    </div>

    <div class="bench">
      <svg class="candle" viewBox="0 0 20 60" focusable="false">
        <rect x="6" y="22" width="8" height="38" rx="1.5" class="wax" />
        <path d="M10 21 C6 15 8 8 10 2 C12 8 14 15 10 21Z" class="flame" />
      </svg>
    </div>

    <div class="warm"></div>
    <div class="motes">
      {#each MOTES as m}
        <span style="left:{m.x}%;--s:{m.size}px;--d:{m.dur}s;--w:{m.delay}s;--x:{m.drift}vmin"></span>
      {/each}
    </div>
    <div class="vignette"></div>
  </div>
</figure>

<style>
  .tale-stage {
    margin: clamp(26px, 4vw, 44px) 0 clamp(30px, 4.4vw, 50px);
    position: relative;
    view-timeline-name: --stage;
    view-timeline-axis: block;
    /* Двойная граница, как у модальных окон дома. */
    border: 1px solid #d8c6b1;
    padding: 5px;
    background: #f8f1e7;
    box-shadow: 0 18px 38px -22px rgba(52, 37, 28, 0.55);
  }

  .room {
    position: relative;
    overflow: hidden;
    aspect-ratio: 4 / 3;
    background: #140e0b;
    isolation: isolate;
    border: 1px solid #6f3b24;
  }

  .room > * {
    position: absolute;
    inset: 0;
  }

  /* ── глубина: стена и окно ───────────────────────────────────────── */

  .wall {
    background:
      radial-gradient(120% 90% at 30% 85%, #3a271b 0%, #22170f 48%, #120c09 100%);
  }

  .far {
    will-change: transform;
  }

  .window {
    position: absolute;
    right: 7%;
    top: 7%;
    width: 27%;
    height: 66%;
    padding: 5px;
    background: #0d0907;
    clip-path: url(#tale-arch);
  }

  .pane {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    clip-path: url(#tale-arch);
    background: linear-gradient(180deg, #4d5d66 0%, #2c3a41 55%, #1a2328 100%);
  }

  .gear {
    position: absolute;
    left: 6%;
    bottom: -12%;
    width: 112%;
    fill: #0f0b09;
    opacity: 0.88;
    animation: turn 150s linear infinite;
  }

  .gear rect {
    transform-box: view-box;
    transform-origin: center;
  }

  .roofs {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 24%;
    background: #0b0807;
    clip-path: polygon(
      0 62%, 8% 62%, 8% 30%, 14% 30%, 14% 62%, 30% 62%, 36% 40%, 42% 62%,
      58% 62%, 58% 18%, 63% 18%, 63% 62%, 80% 62%, 86% 44%, 92% 62%, 100% 62%, 100% 100%, 0 100%
    );
  }

  /* Дождь: две полосы штрихов разной длины и скорости. Рисунок штриха — один,
     наклон в нём и в ходе анимации (-1 : 8) совпадает, иначе дождь «ползёт». */
  .rain {
    position: absolute;
    inset: 0;
    background-repeat: repeat;
  }

  .rain--far {
    opacity: 0.4;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='40' height='80'%3E%3Cg stroke='%23c8d6dc' stroke-width='1' stroke-linecap='round'%3E%3Cpath d='M6 4 L5 22'/%3E%3Cpath d='M24 38 L23 58'/%3E%3Cpath d='M34 10 L33 26'/%3E%3Cpath d='M14 56 L13 74'/%3E%3C/g%3E%3C/svg%3E");
    animation: fall-far 1.15s linear infinite;
  }

  .rain--near {
    opacity: 0.6;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='64' height='120'%3E%3Cg stroke='%23e0ebef' stroke-width='1.4' stroke-linecap='round'%3E%3Cpath d='M10 6 L8 40'/%3E%3Cpath d='M44 62 L42 98'/%3E%3Cpath d='M30 20 L28 44'/%3E%3C/g%3E%3C/svg%3E");
    animation: fall-near 0.7s linear infinite;
  }

  /* ── свет ────────────────────────────────────────────────────────── */

  .glow-back {
    background: radial-gradient(46% 52% at 79% 86%, rgba(198, 95, 60, 0.34), transparent 70%);
    animation: flicker 5.3s ease-in-out infinite;
  }

  .warm {
    pointer-events: none;
    mix-blend-mode: soft-light;
    background: radial-gradient(60% 70% at 79% 90%, rgba(255, 166, 96, 0.75), transparent 72%);
    animation: flicker 5.3s ease-in-out infinite;
  }

  .vignette {
    pointer-events: none;
    background: radial-gradient(90% 90% at 50% 50%, transparent 52%, rgba(6, 3, 2, 0.72) 100%);
  }

  /* ── актёр ───────────────────────────────────────────────────────── */

  /* Три вложенных слоя на один `transform` — по слою на каждое движение:
     два движения одного свойства не складываются, побеждает последнее. */
  .rise {
    inset: auto;
    left: -1%;
    bottom: -3%;
    width: 74%;
  }

  .step,
  .beat {
    position: relative;
    width: 100%;
  }

  .step {
    transform-origin: 40% 100%;
  }

  .beat {
    transform-origin: 38% 92%;
    /* Фотография обрезана по краям кадра: плащ там уходит за границу и
       оставляет прямую вертикаль. Тёмное тело сцены принимает его без шва,
       если край растворить. */
    -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 10%, #000 66%, transparent 98%);
    mask-image: linear-gradient(90deg, transparent 0, #000 10%, #000 66%, transparent 98%);
  }

  .fig,
  .veins {
    display: block;
    width: 100%;
    height: auto;
  }

  .fig {
    filter: brightness(0.6) saturate(0.82);
    transition: filter 1.8s ease 0.2s;
  }

  .veins-wrap {
    position: absolute;
    inset: 0;
    opacity: 0;
    transition: opacity 1.4s ease 0.35s;
  }

  .veins {
    position: absolute;
    inset: 0;
    mix-blend-mode: screen;
  }

  .veins--bloom {
    filter: blur(9px) brightness(1.5) saturate(1.3) hue-rotate(-10deg);
  }

  .awake .fig {
    filter: none;
  }

  .awake .veins-wrap {
    opacity: 1;
  }

  .awake .beat {
    animation: beat 1.9s ease-in-out infinite;
  }

  .awake .veins {
    animation: glow 1.9s ease-in-out infinite;
  }

  /* ── стол и свеча ────────────────────────────────────────────────── */

  .bench {
    top: auto;
    height: 15%;
    background:
      linear-gradient(180deg, rgba(255, 190, 130, 0.18) 0, rgba(255, 190, 130, 0) 6%),
      linear-gradient(180deg, #2b1c12 0%, #1a110b 100%);
    border-top: 1px solid rgba(216, 198, 177, 0.28);
  }

  .candle {
    position: absolute;
    left: 77%;
    bottom: 88%;
    width: 4.6%;
    height: auto;
    overflow: visible;
  }

  .wax {
    fill: #e8dcc6;
  }

  .flame {
    fill: #ffc48a;
    transform-box: fill-box;
    transform-origin: 50% 100%;
    animation: lick 3.1s ease-in-out infinite;
    filter: drop-shadow(0 0 5px rgba(255, 170, 90, 0.9));
  }

  /* ── пыль в луче ─────────────────────────────────────────────────── */

  .motes {
    pointer-events: none;
    container-type: size;
  }

  .motes span {
    position: absolute;
    bottom: -4%;
    width: var(--s);
    height: var(--s);
    border-radius: 50%;
    background: rgba(255, 222, 180, 0.55);
    box-shadow: 0 0 6px rgba(255, 200, 140, 0.5);
    opacity: 0;
    animation: drift var(--d) linear var(--w) infinite;
  }

  /* ── ход ─────────────────────────────────────────────────────────── */

  @keyframes turn {
    to { transform: rotate(360deg); }
  }

  @keyframes fall-far {
    to { background-position: -10px 80px; }
  }

  @keyframes fall-near {
    to { background-position: -16px 120px; }
  }

  @keyframes flicker {
    0%, 100% { opacity: 0.94; }
    22% { opacity: 1; }
    41% { opacity: 0.88; }
    63% { opacity: 0.98; }
    80% { opacity: 0.9; }
  }

  @keyframes lick {
    0%, 100% { transform: scale(1, 1) skewX(0); }
    30% { transform: scale(0.94, 1.07) skewX(2.5deg); }
    55% { transform: scale(1.04, 0.95) skewX(-2deg); }
    78% { transform: scale(0.97, 1.04) skewX(1deg); }
  }

  /* «Тук-тук»: два удара, затем покой. Стук тела и свет жил — одной
     длительности и с одним началом, иначе кажется, что светит не оно. */
  @keyframes beat {
    0%, 14%, 34%, 100% { transform: scale(1); }
    7% { transform: scale(1.011); }
    22% { transform: scale(1.019); }
  }

  @keyframes glow {
    0%, 100% { opacity: 0.26; }
    7% { opacity: 0.85; }
    14% { opacity: 0.3; }
    22% { opacity: 1; }
    38% { opacity: 0.3; }
  }

  @keyframes drift {
    0% { transform: translate(0, 0); opacity: 0; }
    12% { opacity: 0.9; }
    80% { opacity: 0.5; }
    100% { transform: translate(var(--x), -120cqh); opacity: 0; }
  }

  /* ── от прокрутки ────────────────────────────────────────────────── */

  @supports (animation-timeline: view()) {
    @keyframes rise {
      from { transform: translateY(34%); opacity: 0.0; }
      to { transform: translateY(0); opacity: 1; }
    }

    @keyframes step {
      from { transform: scale(0.965); }
      to { transform: scale(1.055); }
    }

    @keyframes deep {
      from { transform: translateY(-3.5%); }
      to { transform: translateY(3.5%); }
    }

    .rise {
      animation: rise linear both;
      animation-timeline: --stage;
      animation-range: entry 8% cover 42%;
    }

    .step {
      animation: step linear both;
      animation-timeline: --stage;
      animation-range: cover 38% exit 38%;
    }

    .far {
      animation: deep linear both;
      animation-timeline: --stage;
      animation-range: entry 0% exit 100%;
    }
  }

  /* ── узкое окно и покой ──────────────────────────────────────────── */

  @media (max-width: 640px) {
    .room { aspect-ratio: 1 / 1; }
    .window { width: 30%; right: 5%; height: 60%; }
    .rise { width: 92%; left: -8%; }
    .candle { left: 80%; width: 6%; }
  }

  @media (prefers-reduced-motion: reduce) {
    .gear,
    .rain,
    .glow-back,
    .warm,
    .flame,
    .rise,
    .step,
    .far,
    .awake .beat,
    .awake .veins {
      animation: none;
    }

    .motes { display: none; }
    .fig { filter: none; transition: none; }
    .veins-wrap { opacity: 1; transition: none; }
    .veins { opacity: 0.55; }
  }
</style>
