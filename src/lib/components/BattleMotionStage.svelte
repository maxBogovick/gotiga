<script lang="ts">
  // Всё нарисованное, что показывает одно событие боя, — и весь словарь
  // движений дома.
  //
  // Компонент нарочно глупый: он не знает ни про события, ни про поводы, ни
  // про карты. Ему дают готовые копии из `stage()` и он их кладёт. Считает
  // `stage()` — один на сцену и на стол хранителя, потому что второй
  // отрисовщик это превью, которое однажды соврёт (`CLAUDE.md`).
  //
  // Здесь же живут `@keyframes` — и живут ГЛОБАЛЬНО (`-global-`), потому что
  // имя движения приходит в инлайновом стиле, который Svelte не переписывает.
  // Это же и удобно: шевеление достаётся телу карты в сцене, а тело карты —
  // не этот компонент.
  import type { StagedMote } from '$lib/battles';

  let { motes = [] }: { motes?: StagedMote[] } = $props();
</script>

<!-- `inset: 0` от поля, а не от доски: печать лежит так же, и
     поле шире доски увело бы всё вбок (`BATTLE-SCENE.md` §10.1). -->
{#if motes.length}
  <div class="motion" aria-hidden="true">
    {#each motes as mote (mote.key)}
      <i class="mote" class:mote--scrap={mote.kind === 'scrap'} style={mote.style}></i>
    {/each}
  </div>
{/if}

<style>
  .motion {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .mote {
    display: block;
    /* Всё остальное — из `stage()`: коробка, картинка, полоса, слой, поворот. */
  }

  /* Кусок самой карты, не спрайт. Цвет бумаги дома, рваный край. Один, не рой. */
  .mote--scrap {
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    box-shadow: 1px 1px 0 #6f3b24;
    clip-path: polygon(10% 8%, 88% 0%, 100% 72%, 68% 100%, 0% 88%, 16% 42%);
  }

  /* ── Словарь жестов ──────────────────────────────────────────────────────
     Список закрыт (`BATTLE-MOTION.md` §3.2): новое движение — это новое
     сочетание жестов, времён и картинок, но никогда не новый жест. Жест обязан
     работать на всех шестнадцати клетках, в обе стороны и при столе вдоль
     комнаты, а такое проверяют, а не настраивают формой. */

  /* Подача к цели и возврат. 55% — это 220 мс из 400: те самые числа, что
     стояли в сцене до движка, и та же кривая. */
  @keyframes -global-gotiga-lunge {
    0% { transform: translate(0, 0); }
    55% { transform: translate(var(--lx, 0), var(--ly, 0)); }
    100% { transform: translate(0, 0); }
  }

  /* Вздрагивание цели: три пиксела и обратно. Не «эффект попадания» — просто
     тело качнулось. */
  @keyframes -global-gotiga-flinch {
    0% { transform: translate(0, 0); }
    40% { transform: translate(3px, 0); }
    100% { transform: translate(0, 0); }
  }

  /* Пробрало без автора: яд, зона, шипы. Мельче вздрагивания и в обе стороны —
     у этого нет направления, потому что нет бьющего. */
  @keyframes -global-gotiga-shiver {
    0% { transform: translate(0, 0); }
    25% { transform: translate(-2px, 0); }
    60% { transform: translate(2px, 0); }
    100% { transform: translate(0, 0); }
  }

  /* Бледнеет и оседает. Ни черепа, ни крестика, ни слова «убит». Держится в
     конце (`both`): клетка пустеет следующим событием, а не этим. */
  @keyframes -global-gotiga-sink {
    0% { opacity: 1; transform: translateY(0); }
    100% { opacity: 0; transform: translateY(6px); }
  }

  /* Залечено: тело чуть подобралось и встало обратно. */
  @keyframes -global-gotiga-rise {
    0% { transform: translateY(0); }
    45% { transform: translateY(-4px); }
    100% { transform: translateY(0); }
  }

  /* Вышло на поле: проявляется. Без полёта из руки — дуга через весь экран
     была бы единственным движением такого размера в доме. */
  @keyframes -global-gotiga-swell {
    0% { opacity: 0; transform: scale(0.96); }
    100% { opacity: 1; transform: scale(1); }
  }

  /* Поклон: наклон и назад. Знамени, котлу, читающему — тому, что не бьёт. */
  @keyframes -global-gotiga-bow {
    0% { transform: rotate(0deg); }
    50% { transform: rotate(-2.5deg); }
    100% { transform: rotate(0deg); }
  }

  /* Замах стрелка: тянется ПРОТИВ цели, потом коротко подаётся к ней.
     `--lx/--ly` смотрят на цель, поэтому «назад» — это те же числа со знаком
     минус, и стрелок с любой клетки натягивает в правильную сторону. */
  @keyframes -global-gotiga-draw {
    0% { transform: translate(0, 0); }
    45% { transform: translate(calc(var(--lx, 0) * -0.7), calc(var(--ly, 0) * -0.7)); }
    62% { transform: translate(calc(var(--lx, 0) * 0.65), calc(var(--ly, 0) * 0.65)); }
    100% { transform: translate(0, 0); }
  }

  /* Отдача: цель отброшена ОТ бьющего. У цели `--lx/--ly` те же, что у
     бьющего, — то есть уже «прочь от него», и умножать ни на что не надо. */
  @keyframes -global-gotiga-recoil {
    0% { transform: translate(0, 0); }
    30% { transform: translate(calc(var(--lx, 0) * 0.55), calc(var(--ly, 0) * 0.55)); }
    100% { transform: translate(0, 0); }
  }

  /* Замах сверху: поднимается и обрушивается. Тяжёлому удару. */
  @keyframes -global-gotiga-heave {
    0% { transform: translateY(0) rotate(0deg); }
    40% { transform: translateY(-12px) rotate(-5deg); }
    62% { transform: translate(calc(var(--lx, 0) * 0.85), calc(var(--ly, 0) * 0.85)) rotate(3deg); }
    100% { transform: translateY(0) rotate(0deg); }
  }

  /* Крупнее и медленнее вздрагивания: досталось всерьёз. */
  @keyframes -global-gotiga-shudder {
    0% { transform: translate(0, 0) rotate(0deg); }
    18% { transform: translate(-6px, 2px) rotate(-2.2deg); }
    44% { transform: translate(6px, -2px) rotate(2.2deg); }
    72% { transform: translate(-3px, 0) rotate(-1deg); }
    100% { transform: translate(0, 0) rotate(0deg); }
  }

  /* Кренится и выпрямляется — медленно. Тому, кто готовит, а не бьёт. */
  @keyframes -global-gotiga-sway {
    0% { transform: rotate(0deg) translateY(0); }
    50% { transform: rotate(-3.4deg) translateY(-5px); }
    100% { transform: rotate(0deg) translateY(0); }
  }

  /* Нависает: подаётся вперёд ростом, а не шагом. */
  @keyframes -global-gotiga-loom {
    0% { transform: scale(1); }
    45% { transform: scale(1.14); }
    100% { transform: scale(1); }
  }

  /* ── Свет ──────────────────────────────────────────────────────────────
     Не вспышка. Вспышка — это язык чужой игры, и он запрещён (§1); свет на
     фотографии — язык этого дома, он уже стоит в `RakingLight` и
     `CandleReveal`. Отсюда и сдержанность чисел: 18% яркости читаются, 80%
     превращают миниатюру в лампу.

     И главное свойство: это `filter`, а не `transform`. Значит, свет можно
     дать телу ВМЕСТЕ с движением — «кренится и светлеет» одним телом. */

  /* Затеплилось: чара. */
  @keyframes -global-gotiga-kindle {
    0% { filter: brightness(1) saturate(1); }
    45% { filter: brightness(1.32) saturate(1.22); }
    100% { filter: brightness(1) saturate(1); }
  }

  /* Краска ушла: холод, оберег, испуг. */
  @keyframes -global-gotiga-blanch {
    0% { filter: saturate(1) brightness(1); }
    40% { filter: saturate(0.2) brightness(1.1); }
    100% { filter: saturate(1) brightness(1); }
  }

  /* Потемнело: проклятие, яд. Не возвращается к концу СВОЕГО жеста — темнеет и
     так остаётся до конца движения, потому что это не мгновение, а то, что
     сделалось. Дальше тело всё равно вернётся в норму: как только событие
     показано, сцена снимает с него стиль целиком. */
  @keyframes -global-gotiga-wither {
    0% { filter: brightness(1) saturate(1); }
    100% { filter: brightness(0.68) saturate(0.5); }
  }

  /* ── Рисунок ──────────────────────────────────────────────────────────── */

  /* Полоса кадров. Конец — не 100%, а `--strip-end`: см. `stripEnd()`, там
     же и арифметика, почему иначе полоса из восьми показывает семь с
     половиной. */
  @keyframes -global-gotiga-strip {
    from { background-position-x: 0%; }
    to { background-position-x: var(--strip-end, 100%); }
  }

  /* Перелёт от бьющего к цели. Поворот несётся внутри кадров: иначе анимация
     `transform` стёрла бы угол, под которым нарисована стрела. */
  @keyframes -global-gotiga-fly {
    from { transform: translate(0, 0) rotate(var(--turn, 0deg)); }
    to { transform: translate(var(--mx, 0), var(--my, 0)) rotate(var(--turn, 0deg)); }
  }

  /* Одиночное оружие на цели: замах, касание, уход. Не полоса кадров и не
     вспышка — клинок проходит карту с той стороны, откуда бьют, и на ударе
     чуть ловит свет, как фотография под косым лучом. */
  @keyframes -global-gotiga-cleave {
    0% {
      transform: rotate(calc(var(--turn, 0deg) - 46deg))
        translate(calc(var(--lx, 0) * -0.85), calc(var(--ly, 0) * -0.85))
        scale(0.72);
      filter: brightness(1) saturate(1);
    }
    38% {
      transform: rotate(calc(var(--turn, 0deg) - 6deg)) translate(0, 0) scale(1.04);
      filter: brightness(1.14) saturate(1.08);
    }
    52% {
      transform: rotate(calc(var(--turn, 0deg) + 10deg))
        translate(calc(var(--lx, 0) * 0.18), calc(var(--ly, 0) * 0.18))
        scale(1.1);
      filter: brightness(1.18) saturate(1.1);
    }
    100% {
      transform: rotate(calc(var(--turn, 0deg) + 28deg))
        translate(calc(var(--lx, 0) * 0.7), calc(var(--ly, 0) * 0.7))
        scale(0.82);
      filter: brightness(1) saturate(1);
    }
  }

  /* Обломок улетает и гаснет. Угол и дальность уже в `--mx/--my` из `stage()`. */
  @keyframes -global-gotiga-scrap {
    from {
      transform: translate(0, 0) rotate(var(--spin0, 0deg));
      opacity: 1;
    }
    to {
      transform: translate(var(--mx, 0), var(--my, 0)) rotate(var(--spin1, 40deg));
      opacity: 0;
    }
  }

  @keyframes -global-gotiga-fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes -global-gotiga-fade-out {
    from { opacity: 1; }
    to { opacity: 0; }
  }

  @keyframes -global-gotiga-fade-inOut {
    0% { opacity: 0; }
    25% { opacity: 1; }
    70% { opacity: 1; }
    100% { opacity: 0; }
  }

  /* Обязательство, а не украшение: при этой настройке движения нет вовсе.
     `stage()` в этом случае и не отдаёт ни одной копии — правило написано
     дважды нарочно, потому что копия может прийти и из стола хранителя. */
  @media (prefers-reduced-motion: reduce) {
    .motion {
      display: none;
    }
  }
</style>
