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

<!-- `inset: 0` от поля, а не от доски: следы ударов и печать лежат так же, и
     поле шире доски увело бы всё вбок (`BATTLE-SCENE.md` §10.1). -->
{#if motes.length}
  <div class="motion" aria-hidden="true">
    {#each motes as mote (mote.key)}
      <i class="mote" style={mote.style}></i>
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
