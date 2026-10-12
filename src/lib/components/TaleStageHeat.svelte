<script lang="ts">
  // Сцена к «Исполню все желания»: всезнающий разум греется.
  //
  // Рисунка в сцене нет, кроме одного прибора. Актёр — фигурка работы
  // (`tools/cutout`), «жар» — трещины с её же лица, у которых оставлен только
  // рисунок (`cracks`). Слов на сцене нет вовсе.
  //
  // Прежние редакции были хуже. Первая — рыбки в миске, записки-наклейки,
  // ниши-силуэты — читалась детской книжкой, а не витриной готических
  // миниатюр. Вторая печатала на стене очередь запросов строками из самого
  // рассказа, то есть повторяла абзацы рядом с абзацами: в исследованиях
  // «занятная, но лишняя деталь» — как раз то, что хуже всего для чтения, а
  // картинка, которая говорит то, что уже сказано, ничего не добавляет. Теперь
  // сцена показывает то, чего в тексте нет: как выглядит жар.
  //
  // Две породы движения:
  //  · от прокрутки — комната нагревается: стена теплеет, трещины разгораются,
  //    стрелка прибора идёт в красную зону (`animation-timeline`, только CSS).
  //    Темп задаёт читатель, а не таймер;
  //  · по времени — трещины неровно пульсируют, стрелка слегка дрожит, фигура
  //    чуть качается. Время стоит, пока сцены нет на экране (`live`).
  //
  // Стоит на полях вместо фото работы (колонка с прилипшей плитой): рядом с
  // текстом, а не в нём, чтобы не разрывать чтение. Поэтому сцена портретная,
  // 3:4, и жар растёт вместе с тем, как читатель идёт вниз по всей байке —
  // шкалу сцене даёт страница (`--heat-timeline`), а без неё сцена меряет себя
  // сама, как на узком экране, где плита падает в текст.
  //
  // Показывается не всем: половине читателей, для замера (`sceneAb.ts`).
  // Для чтения с экрана сцена скрыта. Под `prefers-reduced-motion` она стоит
  // нагретой.
  let {
    figure,
    cracks,
  }: {
    /** Вырезанная фигурка, WebP с прозрачностью. */
    figure: string;
    /** Только трещины той же фотографии, того же размера. */
    cracks: string;
  } = $props();

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

  // Прибор: деления считаются, а не рисуются руками — их сорок одно, и ни одно
  // не должно разойтись с соседом. Шкала от −120° до +120°.
  const TICKS = Array.from({ length: 41 }, (_, i) => {
    const a = ((-120 + i * 6) * Math.PI) / 180;
    const major = i % 5 === 0;
    const r1 = 43;
    const r2 = major ? 36 : 39.5;
    return {
      x1: (Math.sin(a) * r1).toFixed(2),
      y1: (-Math.cos(a) * r1).toFixed(2),
      x2: (Math.sin(a) * r2).toFixed(2),
      y2: (-Math.cos(a) * r2).toFixed(2),
      major,
    };
  });

  // Красная зона — последняя пятая шкалы.
  function arc(from: number, to: number, r: number): string {
    const p = (deg: number) => {
      const a = (deg * Math.PI) / 180;
      return `${(Math.sin(a) * r).toFixed(2)} ${(-Math.cos(a) * r).toFixed(2)}`;
    };
    return `M${p(from)} A${r} ${r} 0 0 1 ${p(to)}`;
  }
  const RED = arc(72, 120, 33);
</script>

<figure class="tale-heat" class:live use:watch aria-hidden="true">
  <div class="hall">
    <div class="wall wall--cold"></div>
    <div class="wall wall--hot"></div>

    <div class="rise">
      <div class="sway">
        <img class="fig" src={figure} alt="" width="1000" height="1170" loading="lazy" decoding="async" />
        <div class="cracks">
          <img class="crack crack--bloom" src={cracks} alt="" width="1000" height="1170" loading="lazy" decoding="async" />
          <img class="crack crack--line" src={cracks} alt="" width="1000" height="1170" loading="lazy" decoding="async" />
        </div>
      </div>
    </div>

    <div class="floor"></div>

    <svg class="gauge" viewBox="-60 -60 120 120" focusable="false">
      <defs>
        <linearGradient id="th-brass" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stop-color="#b8975a" />
          <stop offset="0.45" stop-color="#5a4020" />
          <stop offset="1" stop-color="#9c7c44" />
        </linearGradient>
        <radialGradient id="th-face" cx="0.4" cy="0.35" r="0.8">
          <stop offset="0" stop-color="#f1e6cf" />
          <stop offset="1" stop-color="#cdb98f" />
        </radialGradient>
      </defs>
      <circle r="57" fill="url(#th-brass)" />
      <circle r="51" fill="#1b120b" />
      <circle r="47" fill="url(#th-face)" />
      <path d={RED} fill="none" stroke="#8e2a18" stroke-width="3.2" />
      {#each TICKS as t}
        <line
          x1={t.x1}
          y1={t.y1}
          x2={t.x2}
          y2={t.y2}
          stroke="#2d1f14"
          stroke-width={t.major ? 1.1 : 0.55}
          stroke-linecap="round"
        />
      {/each}
      <g class="needle">
        <g class="needle-shake">
          <path d="M-1.1 7 L0 -37 L1.1 7 Z" fill="#241710" />
          <circle r="3.4" fill="#241710" />
        </g>
      </g>
      <circle r="1.4" fill="#b8975a" />
      <path d="M-34 -22 A40 40 0 0 1 -8 -40" fill="none" stroke="rgba(255,255,255,0.45)" stroke-width="2" stroke-linecap="round" />
    </svg>

    <div class="glow"></div>
    <div class="grain"></div>
    <div class="shade"></div>
  </div>
</figure>

<style>
  .tale-heat {
    margin: 0;
    position: relative;
    view-timeline-name: --heat;
    view-timeline-axis: block;
    border: 1px solid #d8c6b1;
    padding: 5px;
    background: #f8f1e7;
    box-shadow: 0 18px 38px -22px rgba(52, 37, 28, 0.55);
    --run: paused;
  }

  .tale-heat.live {
    --run: running;
  }

  .hall {
    position: relative;
    overflow: hidden;
    aspect-ratio: 3 / 4;
    background: #0b0908;
    isolation: isolate;
    container-type: size;
    border: 1px solid #6f3b24;
  }

  .hall > * {
    position: absolute;
    inset: 0;
  }

  /* ── стена: холодная и горячая, одна поверх другой ───────────────── */

  .wall--cold {
    background: radial-gradient(110% 90% at 28% 62%, #2a2f31 0%, #15191a 55%, #0a0c0c 100%);
  }

  .wall--hot {
    opacity: 0.85;
    background: radial-gradient(110% 90% at 28% 58%, #6a2f18 0%, #381a0e 55%, #150905 100%);
  }

  /* ── фигурка ─────────────────────────────────────────────────────── */

  .rise {
    inset: auto;
    left: -8%;
    bottom: -3%;
    width: 118%;
  }

  .sway {
    position: relative;
    width: 100%;
    transform-origin: 40% 100%;
    animation: sway 6s ease-in-out infinite;
    animation-play-state: var(--run);
  }

  .fig,
  .crack {
    display: block;
    width: 100%;
    height: auto;
  }

  .cracks {
    position: absolute;
    inset: 0;
    opacity: 0.9;
  }

  .crack {
    position: absolute;
    inset: 0;
    mix-blend-mode: screen;
    animation: fever 2.8s ease-in-out infinite;
    animation-play-state: var(--run);
  }

  .crack--bloom {
    filter: blur(8px) brightness(1.5) saturate(1.4);
  }

  /* Низ растворён в темноте: стола нет, фигура уходит в тень, как портрет. */
  .floor {
    inset: auto 0 0 0;
    height: 24%;
    background: linear-gradient(180deg, transparent, #070504 88%);
    pointer-events: none;
  }

  /* ── прибор ──────────────────────────────────────────────────────── */

  .gauge {
    inset: auto 6cqw 7cqh auto;
    width: 38cqw;
    height: auto;
    overflow: visible;
    filter: drop-shadow(0 6px 10px rgba(0, 0, 0, 0.6));
  }

  .needle,
  .needle-shake {
    transform-box: view-box;
    transform-origin: 0 0;
  }

  .needle {
    transform: rotate(70deg);
  }

  .needle-shake {
    animation: tremble 0.22s ease-in-out infinite alternate;
    animation-play-state: var(--run);
  }

  /* ── свет и тень ─────────────────────────────────────────────────── */

  .glow {
    pointer-events: none;
    mix-blend-mode: soft-light;
    opacity: 0.9;
    background: radial-gradient(48% 62% at 30% 48%, rgba(255, 130, 70, 0.95), transparent 72%);
  }

  /* Зерно плёнки: один растр шума, без движения — гравюре нужна фактура, а
     не мерцание. */
  .grain {
    pointer-events: none;
    mix-blend-mode: overlay;
    opacity: 0.22;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='g'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 1.1 -0.2'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23g)'/%3E%3C/svg%3E");
  }

  .shade {
    pointer-events: none;
    background: radial-gradient(90% 90% at 48% 50%, transparent 50%, rgba(5, 3, 2, 0.75) 100%);
  }

  /* ── ход по времени ──────────────────────────────────────────────── */

  @keyframes sway {
    0%, 100% { transform: rotate(-0.5deg); }
    50% { transform: rotate(0.5deg); }
  }

  @keyframes fever {
    0%, 100% { opacity: 0.85; }
    18% { opacity: 1; }
    37% { opacity: 0.55; }
    58% { opacity: 0.95; }
    80% { opacity: 0.7; }
  }

  @keyframes tremble {
    from { transform: rotate(-0.9deg); }
    to { transform: rotate(0.9deg); }
  }

  /* ── от прокрутки: комната нагревается ───────────────────────────── */

  @supports (animation-timeline: view()) {
    @keyframes warm-up {
      from { opacity: 0; }
      to { opacity: 0.85; }
    }

    @keyframes catch-fire {
      from { opacity: 0.05; }
      to { opacity: 0.9; }
    }

    @keyframes glow-up {
      from { opacity: 0; }
      to { opacity: 0.9; }
    }

    @keyframes redline {
      from { transform: rotate(-104deg); }
      to { transform: rotate(80deg); }
    }

    .wall--hot {
      animation: warm-up linear both;
      animation-timeline: var(--heat-timeline, --heat);
      animation-range: var(--heat-range, entry 30% cover 60%);
    }

    .cracks {
      animation: catch-fire linear both;
      animation-timeline: var(--heat-timeline, --heat);
      animation-range: var(--heat-range, entry 30% cover 60%);
    }

    .glow {
      animation: glow-up linear both;
      animation-timeline: var(--heat-timeline, --heat);
      animation-range: var(--heat-range, entry 30% cover 60%);
    }

    .needle {
      animation: redline linear both;
      animation-timeline: var(--heat-timeline, --heat);
      animation-range: var(--heat-range-needle, entry 20% cover 62%);
    }
  }

  /* ── покой ───────────────────────────────────────────────────────── */

  @media (prefers-reduced-motion: reduce) {
    .sway,
    .crack,
    .needle-shake,
    .wall--hot,
    .cracks,
    .glow,
    .needle {
      animation: none;
    }

  }
</style>
