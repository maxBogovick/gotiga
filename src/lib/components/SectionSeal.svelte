<script lang="ts" module>
  /** Край сургучной капли: круг с неровностями, нарисован один раз (зерно 3). */
  export const WAX_PATH =
    'M50.4 26.0 L50.2 28.1 L49.9 30.2 L49.6 32.3 L49.0 34.4 L48.3 36.4 L47.4 38.3 L46.2 40.1 L44.9 41.8 L43.4 43.4 L41.8 44.8 L40.1 46.1 L38.3 47.3 L36.4 48.4 L34.5 49.4 L32.5 50.2 L30.4 50.9 L28.2 51.5 L26.0 51.8 L23.7 51.8 L21.5 51.4 L19.3 50.9 L17.1 50.2 L15.1 49.2 L13.2 48.1 L11.4 46.8 L9.7 45.3 L8.2 43.7 L7.0 41.9 L6.0 39.9 L5.2 37.9 L4.6 35.9 L4.0 33.9 L3.4 32.0 L2.9 30.0 L2.5 28.0 L2.3 26.0 L2.3 23.9 L2.5 21.8 L2.8 19.8 L3.3 17.7 L3.8 15.6 L4.3 13.4 L5.0 11.2 L5.9 9.2 L7.3 7.3 L9.0 5.7 L10.9 4.5 L13.1 3.6 L15.3 3.1 L17.5 2.7 L19.7 2.5 L21.8 2.3 L23.9 2.1 L26.0 2.1 L28.0 2.2 L30.1 2.5 L32.1 2.9 L34.2 3.4 L36.1 4.1 L38.1 4.9 L40.2 5.6 L42.4 6.4 L44.5 7.5 L46.4 8.8 L48.1 10.5 L49.4 12.4 L50.2 14.6 L50.7 17.0 L50.8 19.3 L50.7 21.6 L50.5 23.8Z';
</script>

<script lang="ts">
  // Печать раздела. Без краски — оттиск в бумаге (слепое тиснение); у текущего
  // раздела — сургучная капля. Цвет в ряду печатей один, и он говорит, где
  // человек. Знак внутри — силуэт из `SectionIcon`.
  import { untrack, type Snippet } from 'svelte';
  import SectionIcon, { type SectionIconName } from '$lib/components/SectionIcon.svelte';
  import AppImage from '$lib/components/AppImage.svelte';

  let {
    icon,
    size = 46,
    wax = false,
    photo = null,
    piece,
    art = '',
    artSet = '',
    artStyle = 'free',
    shape = 'round',
    ratio = 0.7,
  }: {
    icon: SectionIconName;
    /** Поперечник оттиска; капля на 8 px шире — сургуч растекается.
     *  Снаружи его можно сменить переменной `--seal-size`. */
    size?: number;
    wax?: boolean;
    /** Фото вместо знака. У такой печати текущий раздел отмечен красной оправой,
     *  а не каплей: капля закрыла бы фото. Пока фото нет, стоит знак и капля. */
    photo?: { src: string; thumbUrl?: string | null } | null;
    /** Предмет вместо знака: книга или карта, целиком и без наклона. */
    piece?: Snippet;
    /** Готовая картинка-медальон с прозрачным фоном (путь от корня сайта). Печать
     *  без бумажного круга: у медальона своя рамка, и круг под ней был бы второй. */
    art?: string;
    /** `srcset` картинки: малая копия для обычного экрана, крупная для плотных. */
    artSet?: string;
    /** `free` — картинка как есть, без бумажного круга; `circle` — фото в круге
     *  печати. В прямоугольной печати картинка всегда заполняет её целиком. */
    artStyle?: 'free' | 'circle';
    /** `rect` — печать-прямоугольник под книгу или карту: круг обрезал бы карту, а
     *  без рамки карта не карта. Форма задана заранее, а не когда пришёл предмет:
     *  иначе пункт менял бы ширину на глазах и ряд, стоящий по центру, дёргался.
     *  У такой печати текущий раздел отмечен оправой, а не каплей. */
    shape?: 'round' | 'rect';
    /** Ширина прямоугольника к высоте. */
    ratio?: number;
  } = $props();

  let rect = $derived(shape === 'rect');
  let ring = $derived(wax && (rect || Boolean(photo) || Boolean(art)));
  let dropWanted = $derived(wax && !rect && !photo && !art);

  // Уникальный id градиента: печатей на странице много, а id — глобальный.
  const gid = `wax-${Math.random().toString(36).slice(2, 8)}`;

  // Капля (второй знак и рисунок сургуча, около 1,8 КБ разметки) рисуется, только
  // когда печать горит или уже горела. Остальные печати ряда её не несут: в HTML
  // каждой страницы их двенадцать, и ни одна из десяти негорящих не показывает её.
  // `lit` включается на кадр позже `armed`, чтобы капля, появившаяся при смене
  // раздела, проявлялась переходом, а не возникала сразу.
  let armed = $state(untrack(() => dropWanted));
  let lit = $state(untrack(() => dropWanted));
  $effect(() => {
    if (!dropWanted) {
      lit = false;
      return;
    }
    if (untrack(() => armed)) {
      lit = true;
      return;
    }
    armed = true;
    requestAnimationFrame(() => requestAnimationFrame(() => (lit = dropWanted)));
  });

  // Картинка сменяется плавно: новая проявляется поверх прежней, и когда она
  // проявилась, прежняя убирается. Первая картинка тоже проявляется, а не
  // выскакивает: она приходит после загрузки страницы.
  type Layer = { key: number; src: string; set: string };
  let layers = $state<Layer[]>([]);
  let layerKey = 0;
  let dropTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const src = art;
    const set = artSet;
    untrack(() => {
      clearTimeout(dropTimer);
      if (!src) {
        layers = [];
        return;
      }
      if (layers.length && layers[layers.length - 1].src === src) return;
      layers = [...layers.slice(-1), { key: ++layerKey, src, set }];
      if (layers.length > 1) {
        dropTimer = setTimeout(() => (layers = layers.slice(-1)), 1000);
      }
    });
    return () => clearTimeout(dropTimer);
  });
</script>

<span class="seal" class:seal--wax={lit} class:seal--ring={ring} class:seal--rect={rect} class:seal--piece={Boolean(piece)} class:seal--art={Boolean(art) && artStyle === 'free'} class:seal--art-circle={Boolean(art) && artStyle === 'circle'} style:--seal-base={`${size}px`} style:--piece-ratio={ratio} aria-hidden="true">
  <span class="blind">
    <SectionIcon name={icon} size={size} />
    {#if piece}
      <span class="piece">{@render piece()}</span>
    {/if}
    {#each layers as layer (layer.key)}
      <img
        class="art"
        src={layer.src}
        srcset={layer.set || undefined}
        sizes={layer.set ? '64px' : undefined}
        alt=""
        decoding="async"
      />
    {/each}
    {#if photo}
      <span class="photo">
        <AppImage src={photo.src} thumbUrl={photo.thumbUrl} alt="" class="photo-img" sizes="64px" loading="eager" />
      </span>
    {/if}
  </span>
  <span class="rim"></span>
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
      inset 1px 1.5px 1.5px rgba(111, 59, 36, 0.16),
      inset -1px -1px 1px rgba(255, 255, 255, 0.9),
      0 0 0 1px rgba(156, 125, 93, 0.4);
    transition: box-shadow 0.25s ease, opacity 0.25s ease, width 0.3s ease, height 0.3s ease;
  }
  .blind :global(.section-icon),
  .drop-icon :global(.section-icon) {
    width: calc(var(--seal) * 0.62);
    height: calc(var(--seal) * 0.62);
    transition: width 0.3s ease, height 0.3s ease;
  }
  .blind :global(.section-icon) {
    position: relative;
    /* ink-secondary на бумаге 10:1; прежние #8f6f52 давали 4.1:1 и тонули в оттиске. */
    color: var(--color-ink-secondary, #5a3420);
    --ib: var(--seal-paper, var(--color-canvas-base, #f8f1e7));
    --em: var(--color-ink-secondary, #5a3420);
    filter: drop-shadow(0 1px 0 rgba(255, 255, 255, 0.9));
    transition: color 0.25s ease, width 0.3s ease, height 0.3s ease;
  }

  /* Наведение на пункт углубляет оттиск — правило стоит у того, кто держит печать. */
  :global(.seal-host:hover) .blind,
  :global(.seal-host:focus-visible) .blind {
    box-shadow:
      inset 1.5px 2px 2.5px rgba(111, 59, 36, 0.26),
      inset -1px -1px 1px rgba(255, 255, 255, 1),
      0 0 0 1px rgba(111, 59, 36, 0.55);
  }
  :global(.seal-host:hover) .blind :global(.section-icon),
  :global(.seal-host:focus-visible) .blind :global(.section-icon) {
    color: var(--color-ink-primary, #2c1710);
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

  /* Фото в круге оттиска: закрывает знак, который стоял под ним до загрузки.
     Объём оттиска возвращает внутренняя тень поверх фото. */
  .photo {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: 50%;
  }
  .photo :global(.photo-img) {
    width: 100%;
    height: 100%;
  }
  .photo :global(.app-image-main),
  .photo :global(.app-image-thumb) {
    object-position: 50% 28%;
  }
  .photo::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.28),
      inset 1px 2px 4px rgba(60, 30, 15, 0.3);
    pointer-events: none;
  }

  /* Медальон: картинка на прозрачном фоне чуть больше печати (острия рамки
     выходят за круг оправы так же, как на самой картинке); бумажного круга под
     ней нет. Знак под картинкой гаснет. */
  .seal--art .blind {
    box-shadow: none !important;
    background: transparent;
    overflow: visible;
  }
  .seal--art .blind :global(.section-icon) {
    opacity: 0;
  }
  @keyframes seal-art-in {
    from {
      opacity: 0;
    }
  }
  .art {
    animation: seal-art-in 0.8s ease both;
    position: absolute;
    left: 50%;
    top: 50%;
    width: calc(var(--seal) * 1.08);
    height: calc(var(--seal) * 1.08);
    transform: translate(-50%, -50%);
    object-fit: contain;
    filter: drop-shadow(0 1px 1.5px rgba(50, 25, 12, 0.45));
    transition: transform 0.25s ease, width 0.3s ease, height 0.3s ease;
    pointer-events: none;
  }
  :global(.seal-host:hover) .art,
  :global(.seal-host:focus-visible) .art {
    transform: translate(-50%, -50%) scale(1.06);
  }

  /* Фото в круге печати: бумажный круг остаётся, картинка заполняет его. */
  .seal--art-circle .blind :global(.section-icon) {
    opacity: 0;
  }
  .seal--art-circle .art {
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    transform: none;
    border-radius: 50%;
    object-fit: cover;
    object-position: 50% 28%;
    filter: none;
  }
  :global(.seal-host:hover) .seal--art-circle .art,
  :global(.seal-host:focus-visible) .seal--art-circle .art {
    transform: none;
  }

  /* Готовая картинка в прямоугольной печати (карта «Игры»): заполняет её целиком,
     углы скруглены под оправу. */
  .seal--rect .art {
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    transform: none;
    object-fit: contain;
    border-radius: 4px;
  }
  :global(.seal-host:hover) .seal--rect .art,
  :global(.seal-host:focus-visible) .seal--rect .art {
    transform: scale(1.05);
  }

  /* Печать-прямоугольник: пункт уже на ширину прямоугольника, а не круга. Пока
     предмета нет, стоит бумажная плашка с знаком; предмет закрывает её целиком. */
  .seal--rect {
    width: calc(var(--seal) * var(--piece-ratio, 0.7) + 8px);
  }
  .seal--rect .blind {
    width: calc(var(--seal) * var(--piece-ratio, 0.7));
    border-radius: 4px;
  }
  .piece {
    position: absolute;
    inset: 0;
    filter: drop-shadow(0 1px 1.5px rgba(50, 25, 12, 0.5));
    transition: transform 0.25s ease;
    pointer-events: none;
  }
  :global(.seal-host:hover) .piece,
  :global(.seal-host:focus-visible) .piece {
    transform: scale(1.05);
  }
  .seal--piece .blind {
    box-shadow: none !important;
    background: transparent;
  }
  .seal--piece .blind :global(.section-icon) {
    opacity: 0;
  }

  /* Красная оправа текущего раздела: круг шире оттиска на 4 px с каждой
     стороны, между фото и оправой 2 px бумаги. Помещается в те же +8 px,
     что занимает капля, поэтому ряд не сдвигается. */
  .rim {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 2px solid var(--color-ember, #c65f3c);
    opacity: 0;
    transform: scale(0.92);
    pointer-events: none;
    transition: opacity 0.25s ease, transform 0.35s cubic-bezier(0.34, 1.4, 0.64, 1);
  }
  .seal--ring .rim {
    opacity: 1;
    transform: none;
  }
  .seal--rect .rim {
    border-radius: 7px;
  }

  .seal--wax .blind {
    opacity: 0;
  }
  .seal--wax .drop {
    opacity: 1;
    transform: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .art {
      animation: none;
    }
    .drop,
    .rim {
      transition: none;
    }
  }
</style>
