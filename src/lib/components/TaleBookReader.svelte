<script lang="ts">
  /**
   * Книга небылицы, раскрытая на полке: обложка откидывается, и читатель
   * листает первые развороты, не уходя со страницы.
   *
   * Это предпросмотр, а не место чтения: отклики, продолжение, сцена на
   * полях, печать «прочитано» и замер чтения живут на странице небылицы
   * (§ 8, § 18). Поэтому книга показывает начало и в конце ведёт туда же.
   *
   * Страница — окно на один столбец общего потока (`columns`): текст течёт
   * сам, и ни размер шрифта, ни язык, ни ширина окна не требуют ручного
   * разбиения. Каждая страница — своя копия потока, сдвинутая на нужный
   * столбец; все копии выкладываются одним и тем же CSS, поэтому разрывы у
   * них совпадают.
   *
   * Листание — настоящий лист: он поворачивается вокруг корешка, у него две
   * стороны (старая правая страница и новая левая), он изгибается (лист
   * нарезан на полосы, каждая повёрнута чуть иначе, чем предыдущая) и
   * отбрасывает тень на страницу под собой. Угол и изгиб пишет один
   * `requestAnimationFrame` в переменные `--theta` и `--bend`; свет и тени
   * считает CSS из этих двух чисел.
   */
  import { onMount, onDestroy, tick } from 'svelte';
  import { t, lang } from '$lib/i18n';
  import { figurineNameIn } from '$lib/workText';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { renderTale, readingMinutes, minutesKey, type TaleBlock } from '$lib/tales';
  import { interact } from '$lib/analytics';
  import type { GazetteLeaf } from '$lib/types/api';
  import TaleProse from './TaleProse.svelte';
  import TaleBookFace from './TaleBookFace.svelte';
  import AppImage from './AppImage.svelte';

  let {
    tale,
    tone = 0,
    onclose,
  }: {
    tale: GazetteLeaf;
    tone?: number;
    onclose: () => void;
  } = $props();

  /** Сколько страниц книги показывается, прежде чем она отсылает на страницу небылицы. */
  const MAX_PAGES = 6;
  /** Сколько знаков текста книга вообще набирает: остальное ей не нужно, а выкладка потока дорога. */
  const TEXT_LIMIT = 9000;
  /** На сколько полос нарезан переворачиваемый лист. */
  const SEGMENTS = 8;
  const TURN_MS = 950;

  let copy = $derived(leafCopy(tale, $lang));
  let photo = $derived(leafCoverUrl(tale));
  let mins = $derived(readingMinutes(copy.body));
  let href = $derived(leafHref(tale, 'tales_book'));

  // Ссылки на дом внутри текста печатаются адресом: названия спрашивает
  // загрузчик страницы небылицы, а не полка. Карточки работ (`card`) в начало
  // книги не берутся.
  let typeset = $derived.by(() => {
    const out: TaleBlock[] = [];
    let chars = 0;
    let dropped = false;
    for (const block of renderTale(copy.body)) {
      if (block.kind === 'card') continue;
      if (chars >= TEXT_LIMIT) {
        dropped = true;
        break;
      }
      if ('runs' in block) {
        chars += block.text.length;
        out.push({
          ...block,
          runs: block.runs.map((r) => (r.kind === 'link' ? { kind: 'text' as const, text: r.ref.bare, bold: r.bold } : r)),
        } as TaleBlock);
      } else {
        out.push(block);
      }
    }
    return { blocks: out, dropped };
  });
  let blocks = $derived(typeset.blocks);

  let opened = $state(false);
  /** Затемнение фона и сама книга на виду; гаснут в самом конце закрытия. */
  let shown = $state(false);
  let swung = $state(false);
  let closing = $state(false);
  let single = $state(false);
  let reduced = $state(false);
  let view = $state(0);
  /** Столбцов текста в потоке. Форзац — страница 0, текст начинается со страницы 1. */
  let textColumns = $state(1);
  let turn = $state<{ dir: 1 | -1 } | null>(null);

  let cols = $derived(single ? 1 : 2);
  let totalPages = $derived(1 + textColumns);
  let allViews = $derived(Math.max(1, Math.ceil(totalPages / cols)));
  let maxViews = $derived(Math.ceil(MAX_PAGES / cols));
  let views = $derived(Math.min(allViews, maxViews));
  let cut = $derived(allViews > maxViews || typeset.dropped);
  let atEnd = $derived(view >= views - 1);
  /** Последняя страница, которую книга показывает: под ней текст тает. */
  let lastPage = $derived(views * cols - 1);

  // Какие страницы стоят в книге сейчас. Пока лист переворачивается, под ним
  // уже лежат новые: вперёд — правая (2v+3), назад — левая (2v−2).
  let leftPage = $derived(turn && turn.dir < 0 ? cols * view - cols : cols * view);
  let rightPage = $derived(turn && turn.dir > 0 ? cols * view + cols + 1 : cols * view + 1);
  let leafFront = $derived(turn ? (turn.dir > 0 ? cols * view + (single ? 0 : 1) : cols * view - 1) : -1);
  let leafBack = $derived(turn ? (single ? -1 : turn.dir > 0 ? cols * view + 2 : cols * view) : -1);
  // Одна страница: под листом лежит соседняя, лист уходит влево и гаснет.
  let soloPage = $derived(turn && turn.dir > 0 ? view + 1 : view);

  let bookEl = $state<HTMLElement | null>(null);
  let flowEl = $state<HTMLElement | null>(null);
  let dialogEl = $state<HTMLElement | null>(null);
  let closeEl = $state<HTMLButtonElement | null>(null);

  async function measure() {
    single = window.matchMedia('(max-width: 820px)').matches;
    await tick();
    if (!flowEl) return;
    const style = getComputedStyle(flowEl);
    const gap = parseFloat(style.columnGap) || 0;
    const count = single ? 1 : 2;
    const colW = (flowEl.clientWidth - gap * (count - 1)) / count;
    textColumns = Math.max(1, Math.round((flowEl.scrollWidth + gap) / (colW + gap)));
    if (view > views - 1) view = views - 1;
  }

  const ease = (k: number) => (k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2);

  async function startTurn(dir: 1 | -1, ms = TURN_MS, whileClosing = false) {
    if (turn || (closing && !whileClosing)) return;
    const next = view + dir;
    if (next < 0 || next >= views) return;
    interact('tale_book_turn', `${tale.slug}:${next + 1}`, next + 1, false);
    if (reduced || !bookEl) {
      view = next;
      return;
    }
    const el = bookEl;
    const maxDeg = single ? 100 : 180;
    // Угол: 0 — лист лежит на правой странице, `maxDeg` — на левой.
    const paint = (k: number) => {
      const p = dir > 0 ? ease(k) : 1 - ease(k);
      el.style.setProperty('--theta', String(p * maxDeg));
      // Свободный край отстаёт от корешка: изгиб растёт к середине поворота и сходит на нет.
      el.style.setProperty('--bend', String(dir * 7 * Math.sin(Math.PI * k)));
      el.style.setProperty('--tfade', single ? String(1 - Math.max(0, (p - 0.7) / 0.3)) : '1');
    };
    // Переменные ставятся раньше, чем лист попадает в разметку: иначе первый
    // кадр был бы листом без поворота.
    paint(0);
    turn = { dir };
    await tick();
    const t0 = performance.now();
    await new Promise<void>((done) => {
      const frame = (now: number) => {
        const k = Math.min(1, (now - t0) / ms);
        paint(k);
        if (k < 1) requestAnimationFrame(frame);
        else done();
      };
      requestAnimationFrame(frame);
    });
    view = next;
    turn = null;
  }

  let alive = true;
  const sleep = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));
  const nextFrame = () => new Promise<void>((done) => requestAnimationFrame(() => done()));

  /**
   * Закрытие — открытие наоборот: страницы перелистываются к началу, крышка
   * ложится на правую половину, книга сдвигается на место, и только потом
   * всё гаснет. Крышка возвращается на то же место, где лежит форзац слева,
   * в тот же кадр, в какой левая страница прячется (как при открытии).
   */
  async function close() {
    if (closing) return;
    closing = true;
    interact('tale_book_close', tale.slug, view + 1, false);
    if (reduced || single || !opened) {
      // Узкое окно и «меньше движения»: книга и фон просто гаснут.
      opened = false;
      shown = false;
      await sleep(reduced ? 0 : 600);
      if (alive) onclose();
      return;
    }
    // Лист, который сейчас переворачивается, доводится до конца.
    while (turn) await sleep(40);
    // К началу — быстрыми листаниями, чтобы под крышкой лежал форзац.
    while (alive && view > 0) await startTurn(-1, 320, true);
    if (!alive) return;
    swung = false;
    await nextFrame();
    await nextFrame();
    opened = false;
    await sleep(1150);
    shown = false;
    await sleep(520);
    if (alive) onclose();
  }

  function onkey(event: KeyboardEvent) {
    if (closing) {
      if (event.key === 'Escape' || event.key.startsWith('Arrow')) event.preventDefault();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      void startTurn(1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      void startTurn(-1);
    } else if (event.key === 'Tab' && dialogEl) {
      const items = [...dialogEl.querySelectorAll<HTMLElement>('button:not([disabled]), a[href]')].filter((el) => el.offsetParent !== null);
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
  }

  let previous = '';
  onMount(() => {
    previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    interact('tale_book_open', tale.slug, null, false);
    void measure();
    void document.fonts?.ready.then(measure);
    window.addEventListener('resize', measure);
    closeEl?.focus();
    // Два кадра: переход срабатывает только от уже отрисованного начального вида.
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        opened = true;
        shown = true;
        if (reduced || single) swung = true;
      }),
    );
  });
  onDestroy(() => {
    alive = false;
    if (typeof document === 'undefined') return;
    document.body.style.overflow = previous;
    window.removeEventListener('resize', measure);
  });

  function leafDone(event: TransitionEvent) {
    if (event.propertyName === 'transform' && opened && !closing) swung = true;
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- Текст страницы: титульный лист и сам рассказ. Один набор на все копии потока. -->
{#snippet flowContent()}
  <header class="titlepage">
    <p class="kick" aria-hidden="true">✦</p>
    <h2 class="book-title">{copy.title}</h2>
    {#if copy.dek}<p class="book-dek">{copy.dek}</p>{/if}
    {#if mins}<p class="book-meta">{mins}&nbsp;{$t(minutesKey(mins))}</p>{/if}
    <p class="rule" aria-hidden="true"></p>
  </header>
  <TaleProse {blocks} />
{/snippet}

{#snippet pastedown()}
  <div class="paste">
    <div class="paste-photo">
      {#if photo}
        <AppImage src={photo} alt="" class="paste-img" sizes="420px" loading="eager" />
      {:else}
        <span class="paste-empty">✦</span>
      {/if}
    </div>
    <p class="paste-name">{figurineNameIn(tale, $lang)}</p>
    <p class="paste-mark" aria-hidden="true">✦ ✦ ✦</p>
  </div>
{/snippet}

<!-- Страница целиком: бумага, поле набора и номер. `side` решает, у какого края тень переплёта. -->
{#snippet pageBox(p: number, side: 'l' | 'r' | 's', face: '' | 'front' | 'back', hidden: boolean)}
  <div class="pb pb--{side}" class:pb--front={face === 'front'} class:pb--back={face === 'back'}>
    {#if p >= 0 && p < totalPages}
      <div class="pbody" class:pbody--fade={cut && p === lastPage} aria-hidden={hidden ? 'true' : undefined}>
        {#if p === 0}
          <div class="frontis">{@render pastedown()}</div>
        {:else}
          <div class="flow" style="--cols: {cols}; --col: {p - 1}">{@render flowContent()}</div>
        {/if}
      </div>
      <span class="folio folio--{side}" aria-hidden="true">{p + 1}</span>
    {/if}
  </div>
{/snippet}

<!-- Полоса листа: две стороны и следующая полоса внутри, повёрнутая чуть иначе. -->
{#snippet seg(i: number)}
  <div class="seg" class:seg0={i === 0} style="--i: {i}">
    <div class="sf">{@render pageBox(leafFront, single ? 's' : 'r', 'front', true)}</div>
    <div class="sb">{@render pageBox(leafBack, single ? 's' : 'l', 'back', true)}</div>
    {#if i < SEGMENTS - 1}{@render seg(i + 1)}{/if}
  </div>
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="veil" class:on={shown} onclick={(e) => e.target === e.currentTarget && close()}>
  <div
    class="stage"
    class:stage--gone={closing && !shown}
    role="dialog"
    aria-modal="true"
    aria-label={copy.title}
    bind:this={dialogEl}
    lang={$lang}
  >
    <div class="book" class:opened class:swung class:single class:turning={!!turn} bind:this={bookEl}>
      <!-- Переплёт и бумага. Левая половина скрыта, пока обложка закрыта. -->
      <div class="block">
        <div class="boards" aria-hidden="true"></div>
        <div class="sheet">
          <div class="pages">
            {#if single}
              {@render pageBox(soloPage, 's', '', false)}
            {:else}
              {@render pageBox(leftPage, 'l', '', false)}
              {@render pageBox(rightPage, 'r', '', false)}
            {/if}
          </div>

          <!-- Тень листа на страницу под ним: справа, пока лист ещё на правой
               половине, слева — когда он перелёг. -->
          {#if turn && !single}
            <div class="cast cast--r" aria-hidden="true"></div>
            <div class="cast cast--l" aria-hidden="true"></div>
          {/if}

          {#if atEnd && !turn && !closing}
            <div class="end">
              <p>{cut ? $t('talesBookContinues') : $t('talesBookEnd')}</p>
              <a class="end-link" {href} onclick={() => interact('tale_book_full', tale.slug, null, false)}>
                {$t('talesBookRead')}
              </a>
            </div>
          {/if}

          <button
            type="button"
            class="edge edge--prev"
            aria-label={$t('talesBookPrev')}
            disabled={view === 0 || !!turn || closing}
            onclick={() => startTurn(-1)}
          ><span aria-hidden="true">‹</span></button>
          <button
            type="button"
            class="edge edge--next"
            aria-label={$t('talesBookNext')}
            disabled={atEnd || !!turn || closing}
            onclick={() => startTurn(1)}
          ><span aria-hidden="true">›</span></button>
        </div>
      </div>

      <!-- Переворачиваемый лист. Есть только пока идёт поворот. -->
      {#if turn}
        <div class="turner" class:turner--single={single} aria-hidden="true">
          {@render seg(0)}
        </div>
      {/if}

      <!-- Крышка: снаружи обложка, изнутри тот же форзац, что стоит слева. -->
      <div class="leaf" aria-hidden="true" ontransitionend={leafDone}>
        <div class="leaf-front">
          <TaleBookFace
            title={copy.title}
            {photo}
            bookCover={tale.bookCover ?? ''}
            author={tale.author ?? ''}
            coverText={tale.bookCoverText}
            {tone}
            sizes="460px"
            eager
          />
        </div>
        <div class="leaf-back">
          <div class="leaf-paper">{@render pastedown()}</div>
        </div>
      </div>
    </div>

    <div class="bar" class:bar--hide={closing}>
      <button type="button" class="bar-btn" bind:this={closeEl} onclick={close}>{$t('talesBookClose')}</button>
      <span class="dots" aria-hidden="true">
        {#each Array(views) as _, i}<i class:on={i === view}></i>{/each}
      </span>
      <a class="bar-btn bar-btn--go" {href} onclick={() => interact('tale_book_full', tale.slug, null, false)}>
        {$t('talesBookRead')}
      </a>
    </div>

    <!-- Невидимый поток для замера: сколько столбцов занимает текст. -->
    <div class="measure" aria-hidden="true">
      <div class="flow" bind:this={flowEl} style="--cols: {cols}; --col: 0">{@render flowContent()}</div>
    </div>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 900;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: rgba(20, 13, 10, 0);
    backdrop-filter: blur(0);
    transition: background 0.6s ease, backdrop-filter 0.6s ease;
  }
  .veil.on {
    background: rgba(20, 13, 10, 0.82);
    backdrop-filter: blur(5px);
  }

  .stage {
    transition: opacity 0.5s ease, transform 0.5s ease;
    --board: 12px;
    --m: clamp(26px, 3.4vw, 46px);
    --top: 34px;
    --bot: 48px;
    --bw: min(1040px, 94vw);
    --bh: min(78svh, 690px);
    /* Ширина страницы, высота и ширина поля набора, зазор между столбцами. */
    --pw: calc((var(--bw) - 2 * var(--board)) / 2);
    --ph: calc(var(--bh) - 2 * var(--board) - var(--top) - var(--bot));
    --cw: calc(var(--pw) - 2 * var(--m));
    --gap: calc(var(--m) * 2);
    --sw: calc(var(--pw) / 8);
    --paper: #f4ebdb;
    --paper-edge: #e6d8be;
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    outline: none;
  }

  .book {
    --theta: 0;
    --bend: 0;
    --tfade: 1;
    position: relative;
    width: var(--bw);
    height: var(--bh);
    perspective: 2600px;
    transform: translateX(-25%);
    transition: transform 1.1s cubic-bezier(0.5, 0.05, 0.25, 1);
    filter: drop-shadow(0 30px 40px rgba(0, 0, 0, 0.55));
  }
  .book.opened { transform: none; }
  /* Книга гаснет, чуть отъезжая и уменьшаясь: закрытая, она уходит со стола. */
  .stage--gone { opacity: 0; transform: translateY(18px) scale(0.94); }
  .bar { transition: opacity 0.3s ease; }
  .bar--hide { opacity: 0; pointer-events: none; }

  /* ── Блок: переплёт и страницы ───────────────────────────────────────── */
  .block {
    position: absolute;
    inset: 0;
    clip-path: inset(0 0 0 50%);
    transition: clip-path 0.45s cubic-bezier(0.2, 0.6, 0.3, 1);
  }
  /* Левая половина открывается, когда крышка уже перешла за корешок: раньше
     она появлялась с первого кадра, и слева стоял пустой коричневый переплёт,
     а крышка ещё лежала справа. При закрытии — зеркало: левая половина
     прячется за первую половину пути крышки, пока та ещё накрывает её, и к
     моменту, когда крышка пересекла корешок, слева уже ничего нет. */
  .book.opened .block {
    clip-path: inset(0 0 0 0);
    transition: clip-path 0.6s cubic-bezier(0.2, 0.6, 0.3, 1) 0.5s;
  }

  .boards {
    position: absolute;
    inset: 0;
    border-radius: 4px 9px 9px 4px;
    background:
      radial-gradient(ellipse 100% 90% at 50% 50%, transparent 50%, rgba(0, 0, 0, 0.4) 100%),
      linear-gradient(160deg, #4d2a22, #2f1914);
    box-shadow: inset 0 0 0 1px rgba(217, 191, 134, 0.22);
  }

  .sheet {
    position: absolute;
    inset: var(--board);
    border-radius: 2px 5px 5px 2px;
    overflow: hidden;
  }
  .pages {
    position: absolute;
    inset: 0;
    display: flex;
  }

  /* ── Страница: бумага, поле набора, номер ─────────────────────────────── */
  .pb {
    position: relative;
    flex: none;
    width: var(--pw);
    height: 100%;
    background:
      url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 .35  0 0 0 0 .25  0 0 0 0 .15  0 0 0 .09 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E"),
      var(--paper);
  }
  /* Тень переплёта у корешка и потемнение у обреза. */
  .pb--r {
    background-image:
      url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 .35  0 0 0 0 .25  0 0 0 0 .15  0 0 0 .09 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E"),
      linear-gradient(90deg, rgba(52, 37, 28, 0.3) 0, rgba(52, 37, 28, 0.1) 3.5%, rgba(52, 37, 28, 0) 12%, rgba(0, 0, 0, 0) 93%, rgba(111, 59, 36, 0.14) 100%);
  }
  .pb--l {
    background-image:
      url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3CfeColorMatrix values='0 0 0 0 .35  0 0 0 0 .25  0 0 0 0 .15  0 0 0 .09 0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E"),
      linear-gradient(270deg, rgba(52, 37, 28, 0.3) 0, rgba(52, 37, 28, 0.1) 3.5%, rgba(52, 37, 28, 0) 12%, rgba(0, 0, 0, 0) 93%, rgba(111, 59, 36, 0.14) 100%);
  }
  .pb--s { box-shadow: inset 0 0 40px rgba(111, 59, 36, 0.1); }

  .pbody {
    position: absolute;
    left: var(--m);
    top: var(--top);
    width: var(--cw);
    height: var(--ph);
    overflow: hidden;
  }
  /* Текст последней показываемой страницы тает под приглашением. */
  .pbody--fade {
    -webkit-mask-image: linear-gradient(to bottom, #000 calc(100% - 120px), transparent calc(100% - 28px));
    mask-image: linear-gradient(to bottom, #000 calc(100% - 120px), transparent calc(100% - 28px));
  }

  /* Поток: столбцы одинаковой ширины. Страница показывает один из них. */
  .flow {
    width: calc(var(--cols) * var(--cw) + (var(--cols) - 1) * var(--gap));
    height: var(--ph);
    column-count: var(--cols);
    column-gap: var(--gap);
    column-fill: auto;
    transform: translateX(calc(var(--col) * (var(--cw) + var(--gap)) * -1));
    font-family: 'Cormorant Garamond', Georgia, serif;
    color: var(--ink, #34251c);
    hyphens: auto;
  }
  .measure {
    position: absolute;
    left: -99999px;
    top: 0;
    visibility: hidden;
    pointer-events: none;
  }

  /* Набор внутри страницы: то же, что делает страница небылицы, но под ширину
     страницы книги, а не окна. */
  .flow :global(.para) {
    font-size: 17.5px;
    line-height: 1.6;
    margin: 0 0 0.85em;
    max-width: none;
    text-align: justify;
    text-wrap: pretty;
    orphans: 2;
    widows: 2;
  }
  .flow :global(.para.opening::first-letter) { font-size: 3.3em; }
  .flow :global(.subhead) { font-size: 26px; margin: 0.6em 0 0.4em; break-after: avoid; }
  .flow :global(.quote) { margin: 1em 0 1.1em; }
  .flow :global(.quote p) { font-size: 22px; }
  .flow :global(.ornament) { margin: 0.2em 0 0.9em; }

  /* ── Форзац ───────────────────────────────────────────────────────────── */
  .frontis { height: 100%; }
  .paste {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 14px;
    text-align: center;
  }
  .paste-photo {
    height: min(64%, 380px);
    aspect-ratio: 3 / 4;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 5px;
    box-sizing: border-box;
    background: linear-gradient(160deg, #e7d19a, #a88445 55%, #d9bf86);
    box-shadow: 0 2px 6px rgba(52, 37, 28, 0.35);
  }
  .paste-photo :global(.paste-img),
  .paste-photo :global(.app-image-wrap) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: sepia(0.28) contrast(0.95) brightness(0.94);
  }
  .paste-photo :global(.app-image-wrap) { background: #120c09; }
  .paste-empty { font-size: 34px; color: rgba(60, 40, 20, 0.6); }
  .paste-name {
    margin: 0;
    font-style: italic;
    font-size: 19px;
    color: var(--muted, #5f4636);
  }
  .paste-mark {
    margin: 0;
    font-size: 11px;
    letter-spacing: 0.5em;
    color: var(--copper, #c65f3c);
    opacity: 0.6;
  }

  /* ── Титульный лист ───────────────────────────────────────────────────── */
  .titlepage { margin: 0 0 1.1em; text-align: center; }
  .kick {
    margin: 4px 0 10px;
    font-size: 13px;
    letter-spacing: 0.5em;
    color: var(--copper, #c65f3c);
    opacity: 0.7;
  }
  .book-title {
    margin: 0 0 10px;
    font-size: clamp(30px, 3.2vw, 40px);
    font-weight: 400;
    line-height: 1.06;
    letter-spacing: -0.01em;
    text-wrap: balance;
    color: var(--ink, #34251c);
  }
  .book-dek {
    margin: 0 0 12px;
    font-style: italic;
    font-size: 17px;
    line-height: 1.45;
    color: var(--muted, #5f4636);
    text-wrap: balance;
  }
  .book-meta {
    margin: 0;
    font: 600 9px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
  }
  .rule {
    width: 46px;
    height: 1px;
    margin: 16px auto 0;
    background: var(--copper, #c65f3c);
    opacity: 0.5;
  }

  .folio {
    position: absolute;
    bottom: 16px;
    font: 500 12px/1 'Cormorant Garamond', Georgia, serif;
    font-variant-numeric: lining-nums;
    letter-spacing: 0.12em;
    color: var(--muted, #5f4636);
    opacity: 0.8;
  }
  .folio--l { left: calc(var(--m) + 2px); }
  .folio--r { right: calc(var(--m) + 2px); }
  .folio--s { left: 50%; transform: translateX(-50%); }

  /* ── Тень листа на страницу под ним ──────────────────────────────────── */
  /* `lead` — какую долю половины листу ещё осталось пройти до края тени. */
  .cast {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 50%;
    pointer-events: none;
    z-index: 2;
  }
  .cast--r {
    --lead: max(cos(calc(var(--theta) * 1deg)), 0);
    left: 50%;
    background: linear-gradient(
      90deg,
      transparent calc(var(--lead) * 100%),
      rgb(24 14 8 / calc(0.5 * sin(calc(var(--theta) * 1deg)))) calc(var(--lead) * 100% + 0.1%),
      transparent calc(var(--lead) * 100% + 34%)
    );
  }
  .cast--l {
    --lead: max(calc(cos(calc(var(--theta) * 1deg)) * -1), 0);
    left: 0;
    background: linear-gradient(
      270deg,
      transparent calc(var(--lead) * 100%),
      rgb(24 14 8 / calc(0.5 * sin(calc(var(--theta) * 1deg)))) calc(var(--lead) * 100% + 0.1%),
      transparent calc(var(--lead) * 100% + 34%)
    );
  }

  /* ── Лист, который переворачивают ─────────────────────────────────────── */
  .turner {
    position: absolute;
    left: 50%;
    top: var(--board);
    width: var(--pw);
    height: calc(100% - 2 * var(--board));
    z-index: 8;
    pointer-events: none;
    transform-style: preserve-3d;
    opacity: var(--tfade);
  }
  .turner--single { left: var(--board); }

  .seg {
    /* Угол полосы у её левого и правого края: правый край этой полосы — левый
       край следующей, поэтому свет переходит без ступеньки. */
    --a0: calc(var(--theta) - var(--i) * var(--bend));
    --a1: calc(var(--theta) - (var(--i) + 1) * var(--bend));
    position: absolute;
    top: 0;
    left: var(--sw);
    /* Полоса чуть шире шага: стык не даёт светлой нити между соседями. */
    width: calc(var(--sw) + 0.6px);
    height: 100%;
    transform-origin: left center;
    transform-style: preserve-3d;
    transform: rotateY(calc(var(--bend) * 1deg));
  }
  .seg0 {
    left: 0;
    transform: rotateY(calc(var(--theta) * -1deg));
    will-change: transform;
  }
  .sf,
  .sb {
    position: absolute;
    inset: 0;
    overflow: hidden;
    backface-visibility: hidden;
    -webkit-backface-visibility: hidden;
  }
  .sb { transform: rotateY(180deg); }
  /* Каждая сторона полосы видит свою часть страницы: передняя — слева направо
     от корешка, задняя — зеркально. */
  .pb--front { margin-left: calc(var(--i) * var(--sw) * -1); }
  .pb--back { margin-left: calc((var(--pw) - (var(--i) + 1) * var(--sw)) * -1); }

  /* Свет: полоса темнеет по синусу угла, под которым она стоит к зрителю. */
  .sf::after,
  .sb::after {
    --d0: calc(0.6 * sin(calc(var(--a0) * 1deg)) * sin(calc(var(--a0) * 1deg)));
    --d1: calc(0.6 * sin(calc(var(--a1) * 1deg)) * sin(calc(var(--a1) * 1deg)));
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(90deg, rgb(28 16 8 / var(--d0)), rgb(28 16 8 / var(--d1)));
  }
  /* Обратная сторона повёрнута на пол-оборота: её левый край — правый край полосы. */
  .sb::after { background: linear-gradient(90deg, rgb(28 16 8 / var(--d1)), rgb(28 16 8 / var(--d0))); }

  .end {
    position: absolute;
    z-index: 5;
    right: var(--m);
    bottom: calc(var(--bot) - 22px);
    width: calc(50% - var(--m) - 4px);
    padding: 40px 0 0;
    text-align: center;
  }
  .book.single .end { right: auto; left: var(--m); width: calc(100% - 2 * var(--m)); }
  .end p {
    margin: 0 0 10px;
    font: italic 400 16px/1.4 'Cormorant Garamond', Georgia, serif;
    color: var(--muted, #5f4636);
  }
  .end-link {
    display: inline-block;
    padding: 9px 18px;
    border-radius: 999px;
    background: var(--ink, #34251c);
    color: #f8f1e7;
    font: 600 11px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    text-decoration: none;
    transition: background 0.25s;
  }
  .end-link:hover { background: var(--copper, #c65f3c); }

  .edge {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 6;
    width: 9%;
    min-width: 34px;
    border: 0;
    background: transparent;
    color: var(--muted, #5f4636);
    font: 300 40px/1 'Cormorant Garamond', Georgia, serif;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.25s, background 0.25s;
  }
  .edge--prev { left: 0; background: linear-gradient(90deg, rgba(52, 37, 28, 0.1), transparent); }
  .edge--next { right: 0; background: linear-gradient(270deg, rgba(52, 37, 28, 0.1), transparent); }
  .edge:hover:not(:disabled),
  .edge:focus-visible:not(:disabled) { opacity: 1; }
  .edge:disabled { cursor: default; }

  /* ── Крышка ───────────────────────────────────────────────────────────── */
  .leaf {
    position: absolute;
    left: 50%;
    top: 0;
    width: 50%;
    height: 100%;
    z-index: 10;
    transform-origin: left center;
    transform-style: preserve-3d;
    transition: transform 1.1s cubic-bezier(0.5, 0.05, 0.25, 1);
    pointer-events: none;
  }
  .book.opened .leaf { transform: rotateY(-180deg); }
  .book.swung .leaf { visibility: hidden; }
  .leaf-front,
  .leaf-back {
    position: absolute;
    inset: 0;
    backface-visibility: hidden;
    -webkit-backface-visibility: hidden;
  }
  .leaf-front { box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4); border-radius: 3px 9px 9px 3px; overflow: hidden; }
  .leaf-back {
    transform: rotateY(180deg);
    border-radius: 9px 3px 3px 9px;
    background: linear-gradient(200deg, #4d2a22, #2f1914);
    padding: var(--board) 0 var(--board) var(--board);
    box-sizing: border-box;
  }
  .leaf-paper {
    position: relative;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, var(--paper-edge), var(--paper) 7%);
    padding: var(--top) var(--m) var(--bot) var(--m);
    box-sizing: border-box;
  }
  .leaf-paper :global(.paste) { height: 100%; }
  /* Левая страница под крышкой появляется в тот же кадр, когда крышка легла и
     скрылась: изнутри крышки стоит тот же форзац. Плавное проявление давало
     провал — крышка уже скрыта, а страница ещё прозрачна, и картинка мигала. */
  .book:not(.swung) .pb--l .pbody,
  .book:not(.swung) .pb--l .folio { opacity: 0; }

  /* ── Нижняя строка ────────────────────────────────────────────────────── */
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    width: var(--bw);
  }
  .bar-btn {
    padding: 9px 16px;
    border-radius: 999px;
    border: 1px solid rgba(244, 235, 219, 0.4);
    background: transparent;
    color: #f4ebdb;
    font: 600 11px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
    transition: background 0.25s, color 0.25s;
  }
  .bar-btn:hover { background: rgba(244, 235, 219, 0.14); }
  .bar-btn--go { background: #f4ebdb; color: #34251c; border-color: #f4ebdb; }
  .bar-btn--go:hover { background: #fff6e2; }
  .bar-btn:focus-visible,
  .end-link:focus-visible,
  .edge:focus-visible { outline: 2px solid #e4cd93; outline-offset: 3px; }

  .dots { display: flex; gap: 8px; }
  .dots i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: rgba(244, 235, 219, 0.3);
    transition: background 0.3s, transform 0.3s;
  }
  .dots i.on { background: #e4cd93; transform: scale(1.3); }

  /* ── Узкое окно: одна страница, без откидывающейся крышки ─────────────── */
  @media (max-width: 820px) {
    .stage {
      --bw: min(500px, 94vw);
      --bh: min(78svh, 720px);
      --m: 24px;
      --pw: calc(var(--bw) - 2 * var(--board));
    }
    .bar-btn { padding: 9px 12px; font-size: 10px; letter-spacing: 0.08em; }
    .book { transform: none; }
    .block { clip-path: none; opacity: 0; transition: opacity 0.5s ease; }
    .book.opened .block { opacity: 1; transition: opacity 0.5s ease; }
    .leaf { display: none; }
    .pb--l { opacity: 1 !important; }
  }

  @media (prefers-reduced-motion: reduce) {
    .book,
    .block,
    .leaf,
    .stage,
    .bar,
    .veil { transition: none; }
  }
</style>
