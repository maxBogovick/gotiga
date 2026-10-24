<script lang="ts">
  /**
   * Обложка книги небылицы.
   *
   * Одна и на полке, и на крышке раскрывающейся книги: две обложки одной
   * небылицы разошлись бы на первой правке. Размер не задаётся — шрифт и
   * поля считаются от ширины самой обложки (`cqi`), поэтому она одинакова и
   * в 240 px на полке, и в 460 px на развороте.
   *
   * Загруженная хозяином готовая картинка (`bookCover`) показывается целиком,
   * а поверх неё стоят надписи, которые он расставил (`coverText`): название и
   * имя автора на матовых табличках. Без картинки обложка собирается: кожа,
   * золотая кайма, окно с фото работы, заглавие тиснением.
   */
  import AppImage from './AppImage.svelte';
  import { labelOf, labelShown, labelStyle, plateStyle, type BookCoverKey } from '$lib/bookCover';
  import type { BookCoverText } from '$lib/types/api';

  let {
    title,
    photo = '',
    bookCover = '',
    tone = 0,
    sizes = '(max-width: 480px) 88vw, (max-width: 860px) 44vw, 260px',
    eager = false,
    author = '',
    coverText = null,
    editing = null,
  }: {
    title: string;
    /** Фото работы для окна-арки. */
    photo?: string;
    /** Готовая обложка, загруженная хозяином. */
    bookCover?: string;
    /** Какая из кож; см. `leatherOf`. */
    tone?: number;
    sizes?: string;
    /** Грузить сразу: обложка на крышке раскрывающейся книги не должна появляться пустой. */
    eager?: boolean;
    /** Имя автора из листа: печатается, если у надписи автора нет своего текста. */
    author?: string;
    /** Надписи на загруженной обложке. */
    coverText?: BookCoverText | null;
    /** Стол хозяина: надписи берутся и переносятся мышью. Читателю не передаётся. */
    editing?: {
      selected: BookCoverKey | null;
      onpick: (key: BookCoverKey) => void;
      onmove: (key: BookCoverKey, x: number, y: number) => void;
      onstart?: () => void;
    } | null;
  } = $props();

  // Кожи приглушены: пергаментный фон сайта не терпит ярких переплётов (§ 1).
  const LEATHER = [
    { base: '#4d2a22', edge: '#2f1914' }, // бордо
    { base: '#2f3b2f', edge: '#1b241c' }, // тёмная зелень
    { base: '#3c2d22', edge: '#241912' }, // умбра
    { base: '#3f2e3b', edge: '#261b23' }, // слива
    { base: '#2d3a40', edge: '#192327' }, // сланец
    { base: '#54402a', edge: '#33261a' }, // охра
  ];
  let skin = $derived(LEATHER[((tone % LEATHER.length) + LEATHER.length) % LEATHER.length]);

  let face = $state<HTMLElement>();

  /** Надписи, которые надо напечатать, в порядке слоёв: автор над названием. */
  let labels = $derived.by(() => {
    if (!bookCover) return [];
    const list: { key: BookCoverKey; label: NonNullable<ReturnType<typeof labelOf>>; text: string }[] = [];
    for (const key of ['title', 'author'] as const) {
      const label = labelOf(coverText, key);
      const text = label?.text.trim() || (key === 'title' ? title : author);
      // Стол показывает и пустую надпись, иначе её нечем взять мышью.
      if (labelShown(label, editing ? text || '·' : text)) list.push({ key, label, text: text || '·' });
    }
    return list;
  });

  let drag: { key: BookCoverKey; px: number; py: number; x: number; y: number } | null = null;

  function take(key: BookCoverKey, x: number, y: number, e: PointerEvent & { currentTarget: HTMLElement }) {
    if (!editing || e.button !== 0) return;
    editing.onstart?.();
    editing.onpick(key);
    e.preventDefault();
    drag = { key, px: e.clientX, py: e.clientY, x, y };
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function carry(e: PointerEvent) {
    if (!drag || !editing || !face) return;
    const box = face.getBoundingClientRect();
    if (!box.width || !box.height) return;
    const clamp = (n: number) => Math.min(100, Math.max(0, Math.round(n * 10) / 10));
    editing.onmove(
      drag.key,
      clamp(drag.x + ((e.clientX - drag.px) / box.width) * 100),
      clamp(drag.y + ((e.clientY - drag.py) / box.height) * 100),
    );
  }

  function drop(e: PointerEvent & { currentTarget: HTMLElement }) {
    drag = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
  }

  /** Стрелки — точная доводка: полпроцента, с Shift — два. */
  function nudge(key: BookCoverKey, x: number, y: number, e: KeyboardEvent) {
    const step = e.shiftKey ? 2 : 0.5;
    const move: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const by = move[e.key];
    if (!by || !editing) return;
    e.preventDefault();
    editing.onstart?.();
    editing.onpick(key);
    editing.onmove(key, Math.min(100, Math.max(0, x + by[0])), Math.min(100, Math.max(0, y + by[1])));
  }
</script>

<span
  class="face"
  class:face--bare={!!bookCover}
  bind:this={face}
  style="--leather: {skin.base}; --leather-edge: {skin.edge}"
>
  {#if bookCover}
    <AppImage src={bookCover} alt={title} class="face-img" {sizes} loading={eager ? 'eager' : 'lazy'} />
    {#each labels as { key, label, text } (key)}
      {#if editing}
        <span
          class="lab lab--editable"
          class:lab--held={editing.selected === key}
          style="{labelStyle(label)};{plateStyle(label)}"
          role="button"
          tabindex="0"
          aria-label={text}
          onpointerdown={(e) => take(key, label.x, label.y, e)}
          onpointermove={carry}
          onpointerup={drop}
          onpointercancel={drop}
          onkeydown={(e) => nudge(key, label.x, label.y, e)}
          onfocus={() => editing.onpick(key)}
        >{text}</span>
      {:else}
        <span class="lab" style="{labelStyle(label)};{plateStyle(label)}" aria-hidden="true">{text}</span>
      {/if}
    {/each}
  {:else}
    <span class="rim" aria-hidden="true"></span>
    <span class="corners" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
    <span class="gilt">
      <span class="window">
        {#if photo}
          <AppImage src={photo} alt="" class="face-photo" {sizes} loading={eager ? 'eager' : 'lazy'} />
        {:else}
          <span class="window-empty" aria-hidden="true">✦</span>
        {/if}
      </span>
    </span>
    <span class="mark" aria-hidden="true">✦</span>
    <span class="title">{title}</span>
  {/if}
  <span class="hinge" aria-hidden="true"></span>
</span>

<style>
  .face {
    position: relative;
    display: block;
    container-type: inline-size;
    width: 100%;
    height: 100%;
    overflow: hidden;
    border-radius: 3px 8px 8px 3px;
    color: #d9bf86;
    background:
      radial-gradient(ellipse 90% 70% at 38% 30%, rgba(255, 235, 200, 0.12), transparent 70%),
      radial-gradient(ellipse 120% 100% at 50% 50%, transparent 55%, rgba(0, 0, 0, 0.42) 100%),
      url("data:image/svg+xml,%3Csvg viewBox='0 0 160 160' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 .55 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E"),
      linear-gradient(160deg, var(--leather), var(--leather-edge));
    background-blend-mode: normal, normal, soft-light, normal;
  }
  .face--bare { background: #1a120e; }
  .face :global(.face-img) { display: block; width: 100%; height: 100%; object-fit: cover; }
  .face :global(.app-image-wrap) { width: 100%; height: 100%; }

  /* Надпись на загруженной обложке: середина — в точке (left, top). Ширина
     включает поля таблички, поэтому `border-box`. */
  .lab {
    position: absolute;
    box-sizing: border-box;
    transform: translate(-50%, -50%);
    overflow-wrap: anywhere;
    text-wrap: balance;
    pointer-events: none;
  }
  .lab--editable {
    pointer-events: auto;
    cursor: grab;
    touch-action: none;
    user-select: none;
    outline: 1px dashed rgba(255, 255, 255, 0.85);
    outline-offset: 2px;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.55);
  }
  .lab--editable:active { cursor: grabbing; }
  .lab--held { outline: 2px solid #c65f3c; outline-offset: 2px; }

  /* Двойная золотая кайма, вытесненная в коже. */
  .rim {
    position: absolute;
    inset: 4.2cqi 4.2cqi 4.2cqi 7.4cqi;
    border: 1px solid rgba(217, 191, 134, 0.7);
    box-shadow: 0 0 0 0.9cqi transparent, inset 0 0 0 0.9cqi transparent;
    outline: 1px solid rgba(217, 191, 134, 0.32);
    outline-offset: 1.1cqi;
    border-radius: 1px;
  }

  /* Угловые ромбы на кайме. */
  .corners i {
    position: absolute;
    width: 2.6cqi;
    height: 2.6cqi;
    background: #d9bf86;
    transform: rotate(45deg);
    opacity: 0.85;
  }
  .corners i:nth-child(1) { left: calc(7.4cqi - 1.3cqi); top: calc(4.2cqi - 1.3cqi); }
  .corners i:nth-child(2) { right: calc(4.2cqi - 1.3cqi); top: calc(4.2cqi - 1.3cqi); }
  .corners i:nth-child(3) { left: calc(7.4cqi - 1.3cqi); bottom: calc(4.2cqi - 1.3cqi); }
  .corners i:nth-child(4) { right: calc(4.2cqi - 1.3cqi); bottom: calc(4.2cqi - 1.3cqi); }

  /* Окно с фото: прямоугольник в золотой рамке. Толщина рамки в `cqi`
     растёт вместе с обложкой. */
  .gilt {
    position: absolute;
    left: 50%;
    top: 12cqi;
    width: 56cqi;
    margin-left: calc(3.2cqi - 28cqi);
    aspect-ratio: 3 / 4;
    padding: 1.1cqi;
    background: linear-gradient(160deg, #e7d19a, #a88445 55%, #d9bf86);
    border-radius: 1px;
    box-sizing: border-box;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }
  .window {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: #120c09;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.55);
  }
  .window :global(.face-photo) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: sepia(0.32) contrast(0.96) brightness(0.9);
  }
  .window :global(.app-image-wrap) { width: 100%; height: 100%; }
  .window-empty { font-size: 9cqi; color: rgba(217, 191, 134, 0.5); }

  .mark {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(12cqi + 56cqi * 4 / 3 + 4.2cqi);
    text-align: center;
    padding-left: 6.4cqi;
    font-size: 4.6cqi;
    opacity: 0.8;
  }

  .title {
    position: absolute;
    left: 14cqi;
    right: 9cqi;
    top: calc(12cqi + 56cqi * 4 / 3 + 11.5cqi);
    text-align: center;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 8.4cqi;
    font-weight: 500;
    line-height: 1.08;
    letter-spacing: 0.01em;
    text-wrap: balance;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    color: #e4cd93;
    text-shadow: 0 -1px 0 rgba(0, 0, 0, 0.55), 0 1px 0 rgba(255, 238, 190, 0.18);
  }

  /* Корешок: тёмная полоса с бороздой у переплёта. */
  .hinge {
    position: absolute;
    inset: 0 auto 0 0;
    width: 9cqi;
    pointer-events: none;
    background:
      linear-gradient(90deg, rgba(0, 0, 0, 0.5) 0, rgba(0, 0, 0, 0.18) 55%, rgba(0, 0, 0, 0) 100%),
      linear-gradient(90deg, transparent 5.6cqi, rgba(0, 0, 0, 0.45) 6.2cqi, rgba(255, 236, 190, 0.16) 6.9cqi, transparent 7.6cqi);
  }
</style>
