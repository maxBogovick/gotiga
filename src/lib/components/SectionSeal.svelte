<script lang="ts" module>
  /** Край сургучной капли: круг с неровностями, нарисован один раз (зерно 3). */
  export const WAX_PATH =
    'M50.4 26.0 L50.2 28.1 L49.9 30.2 L49.6 32.3 L49.0 34.4 L48.3 36.4 L47.4 38.3 L46.2 40.1 L44.9 41.8 L43.4 43.4 L41.8 44.8 L40.1 46.1 L38.3 47.3 L36.4 48.4 L34.5 49.4 L32.5 50.2 L30.4 50.9 L28.2 51.5 L26.0 51.8 L23.7 51.8 L21.5 51.4 L19.3 50.9 L17.1 50.2 L15.1 49.2 L13.2 48.1 L11.4 46.8 L9.7 45.3 L8.2 43.7 L7.0 41.9 L6.0 39.9 L5.2 37.9 L4.6 35.9 L4.0 33.9 L3.4 32.0 L2.9 30.0 L2.5 28.0 L2.3 26.0 L2.3 23.9 L2.5 21.8 L2.8 19.8 L3.3 17.7 L3.8 15.6 L4.3 13.4 L5.0 11.2 L5.9 9.2 L7.3 7.3 L9.0 5.7 L10.9 4.5 L13.1 3.6 L15.3 3.1 L17.5 2.7 L19.7 2.5 L21.8 2.3 L23.9 2.1 L26.0 2.1 L28.0 2.2 L30.1 2.5 L32.1 2.9 L34.2 3.4 L36.1 4.1 L38.1 4.9 L40.2 5.6 L42.4 6.4 L44.5 7.5 L46.4 8.8 L48.1 10.5 L49.4 12.4 L50.2 14.6 L50.7 17.0 L50.8 19.3 L50.7 21.6 L50.5 23.8Z';
</script>

<script lang="ts">
  // Печать раздела. Без краски — оттиск в бумаге (слепое тиснение); у текущего
  // раздела — сургучная капля. Цвет в ряду печатей один, и он говорит, где
  // человек. Знак внутри — силуэт из `SectionIcon`.
  import { untrack } from 'svelte';
  import SectionIcon, { type SectionIconName } from '$lib/components/SectionIcon.svelte';

  let {
    icon,
    size = 46,
    wax = false,
  }: {
    icon: SectionIconName;
    /** Поперечник оттиска; капля на 8 px шире — сургуч растекается.
     *  Снаружи его можно сменить переменной `--seal-size`. */
    size?: number;
    wax?: boolean;
  } = $props();

  // Уникальный id градиента: печатей на странице много, а id — глобальный.
  const gid = `wax-${Math.random().toString(36).slice(2, 8)}`;

  // Капля (второй знак и рисунок сургуча, около 1,8 КБ разметки) рисуется, только
  // когда печать горит или уже горела. Остальные печати ряда её не несут: в HTML
  // каждой страницы их двенадцать, и ни одна из десяти негорящих не показывает её.
  // `lit` включается на кадр позже `armed`, чтобы капля, появившаяся при смене
  // раздела, проявлялась переходом, а не возникала сразу.
  let armed = $state(untrack(() => wax));
  let lit = $state(untrack(() => wax));
  $effect(() => {
    if (!wax) {
      lit = false;
      return;
    }
    if (untrack(() => armed)) {
      lit = true;
      return;
    }
    armed = true;
    requestAnimationFrame(() => requestAnimationFrame(() => (lit = wax)));
  });
</script>

<span class="seal" class:seal--wax={lit} style:--seal-base={`${size}px`} aria-hidden="true">
  <span class="blind"><SectionIcon name={icon} size={size} /></span>
  {#if armed}
  <span class="drop">
    <svg viewBox="0 0 52 52">
      <defs>
        <radialGradient id={gid} cx=".42" cy=".38" r=".7">
          <stop offset="0" stop-color="#b65332" />
          <stop offset=".7" stop-color="#963a20" />
          <stop offset="1" stop-color="#7c2e18" />
        </radialGradient>
      </defs>
      <path d={WAX_PATH} fill="url(#{gid})" />
      <circle cx="26" cy="26" r="17.5" fill="none" stroke="#6f2512" stroke-width="1" opacity=".55" />
      <circle cx="26" cy="26" r="16.6" fill="none" stroke="#d27c58" stroke-width=".6" opacity=".4" />
    </svg>
    <span class="drop-icon"><SectionIcon name={icon} size={size} /></span>
  </span>
  {/if}
</span>

<style>
  /* Поперечник можно сменить снаружи (`--seal-size`): свёрнутая прокруткой
     шапка уменьшает печати, не пересобирая их. Знак — доля печати. */
  .seal {
    --seal: var(--seal-size, var(--seal-base));
    position: relative;
    display: grid;
    place-items: center;
    width: calc(var(--seal) + 8px);
    height: calc(var(--seal) + 8px);
    flex: none;
    transition: width 0.3s ease, height 0.3s ease;
  }

  /* Оттиск: бумага того же цвета, свет сверху слева, тень в выемке снизу справа. */
  .blind {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--seal);
    height: var(--seal);
    border-radius: 50%;
    background: var(--seal-paper, var(--color-canvas-base, #f8f1e7));
    box-shadow:
      inset 1.5px 2px 2px rgba(111, 59, 36, 0.22),
      inset -1.5px -1.5px 1.5px rgba(255, 255, 255, 0.95),
      0 0 0 1px rgba(156, 125, 93, 0.28);
    transition: box-shadow 0.25s ease, opacity 0.25s ease, width 0.3s ease, height 0.3s ease;
  }
  .blind::before {
    content: '';
    position: absolute;
    inset: 4px;
    border-radius: 50%;
    box-shadow:
      inset 0 0 0 0.75px rgba(156, 125, 93, 0.45),
      0 0.75px 0 rgba(255, 255, 255, 0.8);
  }
  .blind :global(.section-icon),
  .drop-icon :global(.section-icon) {
    width: calc(var(--seal) * 0.54);
    height: calc(var(--seal) * 0.54);
    transition: width 0.3s ease, height 0.3s ease;
  }
  .blind :global(.section-icon) {
    position: relative;
    color: #8f6f52;
    --ib: var(--seal-paper, var(--color-canvas-base, #f8f1e7));
    --em: #8f6f52;
    filter: drop-shadow(0 1px 0 rgba(255, 255, 255, 0.95)) drop-shadow(0 -0.6px 0 rgba(90, 52, 32, 0.35));
    transition: color 0.25s ease, width 0.3s ease, height 0.3s ease;
  }

  /* Наведение на пункт углубляет оттиск — правило стоит у того, кто держит печать. */
  :global(.seal-host:hover) .blind,
  :global(.seal-host:focus-visible) .blind {
    box-shadow:
      inset 2px 2.5px 3px rgba(111, 59, 36, 0.32),
      inset -1.5px -1.5px 1.5px rgba(255, 255, 255, 1),
      0 0 0 1px rgba(111, 59, 36, 0.45);
  }
  :global(.seal-host:hover) .blind :global(.section-icon),
  :global(.seal-host:focus-visible) .blind :global(.section-icon) {
    color: #7a5035;
  }

  .drop {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    opacity: 0;
    transform: scale(0.85) rotate(-8deg);
    transition: opacity 0.25s ease, transform 0.35s cubic-bezier(0.34, 1.4, 0.64, 1);
    filter: drop-shadow(0 2px 2px rgba(80, 30, 12, 0.35));
  }
  .drop svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .drop-icon {
    position: relative;
    display: grid;
  }
  .drop-icon :global(.section-icon) {
    color: #6a2210;
    --ib: #a2442a;
    --em: #e9a07a;
    filter: drop-shadow(0 1px 0 rgba(240, 170, 140, 0.35));
  }

  .seal--wax .blind {
    opacity: 0;
  }
  .seal--wax .drop {
    opacity: 1;
    transform: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .drop {
      transition: none;
    }
  }
</style>
