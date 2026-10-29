<script lang="ts">
  /**
   * Книга небылицы, раскрытая на полке: обложка откидывается, и читатель
   * листает первые развороты, не уходя со страницы.
   *
   * Книга показывает весь текст небылицы. Отклики, продолжение, сцена на
   * полях, печать «прочитано» и замер чтения живут на странице небылицы
   * (§ 8, § 18), поэтому ссылка туда стоит в нижней строке.
   *
   * Листать можно нажатием в любое место страницы: левая половина книги
   * листает назад, правая — вперёд (`.edge`), а не только узкая полоса у обреза.
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
  import { t, lang, locale } from '$lib/i18n';
  import { figurineNameIn } from '$lib/workText';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { renderTale, readingMinutes, minutesKey, type TaleBlock } from '$lib/tales';
  import { interact } from '$lib/analytics';
  import { authStore } from '$lib/stores/auth.svelte';
  import { visitorToken } from '$lib/visitorToken';
  import { api } from '$lib/api';
  import { resolveSiteRefs, siteRefsIn, type SiteRefInfo } from '$lib/siteLinks';
  import ArchClip from './ArchClip.svelte';
  import { sceneFromData, taleSceneFor, type TaleScene } from '$lib/taleScenes';
  import type { CommentDto, GazetteLeaf } from '$lib/types/api';

  type Frame = { url: string; position?: string };
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

  /** На сколько полос нарезан переворачиваемый лист. */
  const SEGMENTS = 8;
  const TURN_MS = 950;
  const FONT_WAIT_MS = 1500;

  let copy = $derived(leafCopy(tale, $lang));
  let photo = $derived(leafCoverUrl(tale));
  let mins = $derived(readingMinutes(copy.body));
  let href = $derived(leafHref(tale, 'tales_book'));
  let workHref = $derived(tale.figurineSlug || tale.figurineId ? `/figurines/${tale.figurineSlug || tale.figurineId}` : null);

  // Ссылки на дом внутри текста — настоящие ссылки; названия им спрашивает сама
  // книга (полка читает список без тел, а тут нужны названия на языке читателя).
  let blocks = $derived(renderTale(copy.body));
  let links = $state<Record<string, SiteRefInfo>>({});

  // Кадры сцены этой байки стоят в тексте там, где их ставят настройки сцены:
  // кадр `k` приходит на переходе `k − 1`, момент `at` — доля чтения по знакам.
  // Выключенной или пустой сцены нет, и книга печатается без картинок.
  let scene = $state<TaleScene | null>(null);

  function blockChars(b: TaleBlock): number {
    return 'runs' in b ? b.runs.reduce((n, r) => n + (r.kind === 'text' ? r.text.length : 0), 0) : 0;
  }

  /** Кадры перед текстом и кадры после блока с данным номером. */
  let figures = $derived.by(() => {
    const before: Frame[] = [];
    const afterBlock = new Map<number, Frame[]>();
    const frames = scene?.frames ?? [];
    if (!scene || !frames.length) return { before, afterBlock };
    const sizes = blocks.map(blockChars);
    const total = sizes.reduce((a, b) => a + b, 0) || 1;
    // Ни один кадр не встаёт раньше первого абзаца: кадр 0 (момент 0) идёт сразу за ним.
    const firstPara = Math.max(0, blocks.findIndex((b) => b.kind === 'p'));
    frames.forEach((frame, k) => {
      const at = k === 0 ? 0 : (scene!.transitions?.[k - 1]?.at ?? 1);
      let end = 0;
      let at_i = blocks.length - 1;
      for (let i = 0; i < blocks.length; i++) {
        end += sizes[i];
        if (end / total >= at) {
          at_i = i;
          break;
        }
      }
      at_i = Math.max(at_i, firstPara);
      afterBlock.set(at_i, [...(afterBlock.get(at_i) ?? []), frame]);
    });
    return { before, afterBlock };
  });

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
  let views = $derived(Math.max(1, Math.ceil(totalPages / cols)));
  let atEnd = $derived(view >= views - 1);

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
    const mark = flowEl.querySelector<HTMLElement>('.guests');
    guestPage = mark
      ? Math.floor((mark.getBoundingClientRect().left - flowEl.getBoundingClientRect().left + 1) / (colW + gap)) + 1
      : -1;
    if (view > views - 1) view = views - 1;
  }

  // ── Книга отзывов: одобренные записи читателей стоят листами после текста ──
  let guests = $state<CommentDto[]>([]);
  /** Страница (с нуля, форзац — 0), на которой начинаются отзывы; −1 — отзывов нет. */
  let guestPage = $state(-1);
  let jumping = false;

  async function loadGuests() {
    const list = await api.getTaleComments(tale.id).catch(() => [] as CommentDto[]);
    if (!alive) return;
    guests = list;
    // Всегда: ответ мог прийти между замером в `start()` и открытием, и тогда число страниц и место отзывов остались бы прежними.
    void measure();
  }

  function guestDate(iso: string): string {
    return new Date(iso).toLocaleDateString($locale, { year: 'numeric', month: 'long', day: 'numeric' });
  }

  /** Закладка: к первому листу отзывов, быстрыми поворотами. */
  async function toGuests() {
    if (jumping || guestPage < 0 || turn || closing) return;
    const target = Math.floor(guestPage / cols);
    jumping = true;
    interact('tale_book_ribbon', tale.slug, null, false);
    try {
      while (view < target && !closing && alive) {
        const before = view;
        await startTurn(1, 280);
        if (view === before) break;
      }
    } finally {
      jumping = false;
    }
  }

  // ── Последняя страница: отметить, вписать впечатление, поделиться ─────────
  let myVote = $state<0 | 1 | -1>(0);
  let likes = $state(0);
  let voting = $state(false);
  let needLogin = $state(false);
  let note = $state('');
  let noteName = $state('');
  let noteSending = $state(false);
  let noteSent = $state(false);
  let noteError = $state('');
  let copied = $state(false);
  let stamped = $state(0);

  // Недописанная запись переживает закрытие книги: лежит в браузере, пока её не отправили.
  const draftKey = () => `gotiga_tale_note_${tale.id}`;
  function loadDraft() {
    try {
      const d = JSON.parse(localStorage.getItem(draftKey()) ?? 'null');
      if (d && typeof d === 'object') {
        note = typeof d.note === 'string' ? d.note : '';
        noteName = typeof d.name === 'string' ? d.name : '';
      }
    } catch {
      // Хранилище закрыто — запись живёт, пока открыта книга.
    }
  }
  function saveDraft() {
    try {
      if (note || noteName) localStorage.setItem(draftKey(), JSON.stringify({ note, name: noteName }));
      else localStorage.removeItem(draftKey());
    } catch {
      // см. выше
    }
  }

  async function loadStats() {
    const stats = await api.getTaleStats(tale.id, visitorToken());
    if (!alive || !stats) return;
    likes = stats.likes;
    myVote = (stats.myVote ?? 0) as 0 | 1 | -1;
  }

  /** Как на странице байки: голос назначается, сервер записывает названное. */
  async function like() {
    if (voting) return;
    if (!authStore.isLoggedIn) {
      needLogin = true;
      return;
    }
    needLogin = false;
    const value = myVote === 1 ? 0 : 1;
    const wasVote = myVote;
    const wasLikes = likes;
    myVote = value;
    likes = Math.max(0, likes + (value === 1 ? 1 : 0) - (wasVote === 1 ? 1 : 0));
    if (value === 1) stamped++;
    voting = true;
    interact('tale_book_like', tale.slug, value, false);
    try {
      const out = await api.setTaleVote(tale.id, visitorToken(), value, authStore.token);
      myVote = out.value as 0 | 1 | -1;
      likes = out.likes;
    } catch {
      myVote = wasVote;
      likes = wasLikes;
    } finally {
      voting = false;
    }
  }

  async function writeNote(event: SubmitEvent) {
    event.preventDefault();
    const body = note.trim();
    if (!body || noteSending) return;
    if (!authStore.isLoggedIn && !noteName.trim()) {
      noteError = $t('commentsNameError');
      return;
    }
    noteError = '';
    noteSending = true;
    try {
      await api.submitTaleComment(
        tale.id,
        { authorName: authStore.isLoggedIn ? undefined : noteName.trim(), body },
        authStore.token,
      );
      noteSent = true;
      note = '';
      saveDraft();
      interact('tale_book_note', tale.slug, null, false);
    } catch (err) {
      noteError = err instanceof Error ? err.message : 'Error';
    } finally {
      noteSending = false;
    }
  }

  async function share() {
    const url = `${location.origin}/tales/${tale.slug}?src=book_share`;
    interact('tale_book_share', tale.slug, null, false);
    try {
      if (navigator.share) {
        await navigator.share({ title: copy.title, text: copy.dek || undefined, url });
        return;
      }
    } catch (err) {
      if ((err as Error)?.name === 'AbortError') return;
    }
    try {
      await navigator.clipboard.writeText(url);
      copied = true;
      setTimeout(() => (copied = false), 2400);
    } catch {
      // Не скопировалось — ничего не обещаем.
    }
  }

  // Края листания лежат поверх страниц. Пока под указателем ссылка или кнопка
  // последней страницы, края пропускают нажатие сквозь себя, и оно доходит до
  // неё настоящим: ссылка идёт по адресу, в поле встаёт курсор.
  let overHit = $state(false);
  function probe(event: PointerEvent) {
    overHit = document
      .elementsFromPoint(event.clientX, event.clientY)
      .some((el) => !!el.closest('a[href], .hit') && !!el.closest('.flow, .frontis'));
  }

  /**
   * Края листания лежат поверх страниц, поэтому нажатие по ссылке в тексте
   * приходит сначала им: если под указателем ссылка — идём по ней, иначе листаем.
   */
  function pageClick(event: MouseEvent, dir: 1 | -1) {
    const link = document
      .elementsFromPoint(event.clientX, event.clientY)
      .find((el): el is HTMLAnchorElement => el instanceof HTMLAnchorElement && !!el.closest('.flow, .frontis') && el.offsetParent !== null);
    if (link) {
      link.click();
      return;
    }
    void startTurn(dir);
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
    // К началу — быстрыми листаниями, чтобы под крышкой лежал форзац; из
    // глубины длинной книги сначала прыжок, иначе закрытие шло бы десятки листаний.
    if (view > 2) view = 2;
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
    const typing = event.target instanceof HTMLElement && /^(TEXTAREA|INPUT)$/.test(event.target.tagName);
    if (typing && event.key !== 'Escape') return;
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
      const items = [...dialogEl.querySelectorAll<HTMLElement>('button:not([disabled]), a[href], textarea, input')].filter((el) => el.offsetParent !== null);
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
    window.addEventListener('resize', measure);
    closeEl?.focus();
    void start();
    loadDraft();
    void loadStats();
  });

  /**
   * Шрифт текста на полке не используется, поэтому к открытию книги он ещё не
   * загружен: страницы встали бы на запасном шрифте, и текст перестраивался бы
   * прямо во время откидывания обложки. Шрифт ждётся до начала открытия (не
   * дольше `FONT_WAIT_MS`: медленная сеть не должна держать книгу закрытой).
   */
  async function start() {
    // Сцена не задерживает открытие дольше слова о шрифте; опоздавшая — перестраивает страницы.
    const refs = siteRefsIn([tale.bodyEn, tale.bodyRu, tale.bodyUk, tale.bodyEs, tale.bodyDe, tale.bodyIt].map((b) => b ?? '').join('\n'));
    if (refs.length) {
      void resolveSiteRefs(refs, {
        getFigurine: (handle) => api.getFigurine(handle),
        getGazetteLeaf: (slug) => api.getGazetteLeaf(slug),
      }).then((named) => {
        if (!alive) return;
        links = named;
        void measure();
      });
    }
    const guestLoad = loadGuests();
    const sceneLoad = api
      .getTaleScene(tale.id)
      .then(async (s) => {
        if (!alive) return;
        // Как на странице байки: сцена из базы главнее сцены из кода.
        const own = s?.enabled && s.frames.length ? sceneFromData(s) : taleSceneFor(tale.slug);
        scene = own?.frames?.length ? own : null;
        // Картинки прогреваются заранее: страница, смонтированная при листании,
        // иначе показывает пустую рамку и проявляет кадр с опозданием (Safari).
        const warm = (scene?.frames ?? []).map((f) => {
          const img = new Image();
          img.src = f.url;
          return img.decode().catch(() => {});
        });
        await Promise.race([Promise.all(warm), sleep(FONT_WAIT_MS)]);
        if (!alive) return;
        void measure();
      })
      .catch(() => {});
    const sample = `${copy.title} ${copy.dek ?? ''} ${copy.body}`;
    const faces = ['400 17px', 'italic 400 17px', '500 12px'].map((f) => document.fonts?.load(`${f} 'Cormorant Garamond'`, sample));
    await Promise.race([Promise.allSettled([...faces, sceneLoad, guestLoad]), sleep(FONT_WAIT_MS)]);
    if (!alive) return;
    await measure();
    // Два кадра: переход срабатывает только от уже отрисованного начального вида.
    await nextFrame();
    await nextFrame();
    if (!alive) return;
    opened = true;
    shown = true;
    if (reduced || single) swung = true;
  }
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
  {#each figures.before as frame}{@render figure(frame)}{/each}
  <TaleProse {blocks} {links}>
    {#snippet after(i)}
      {#each figures.afterBlock.get(i) ?? [] as frame}{@render figure(frame)}{/each}
    {/snippet}
  </TaleProse>
  <p class="the-end">✦ {$t('talesBookEnd')}</p>
  {@render colophon()}
  {@render guestbook()}
{/snippet}

<!-- Книга отзывов: листы гостей. Начинается с новой страницы, запись не рвётся пополам. -->
{#snippet guestbook()}
  {#if guests.length}
    <section class="guests">
      <p class="colo-orn" aria-hidden="true">❦</p>
      <h3 class="guests-title">{$t('commentsTitle')}</h3>
      <ol class="entries">
        {#each guests as c (c.id)}
          <li class="entry">
            <p class="entry-body">{c.body}</p>
            <p class="entry-sign">— {c.authorName}<span class="entry-date">, <time datetime={c.createdAt}>{guestDate(c.createdAt)}</time></span></p>
            {#if c.adminReply}
              <div class="entry-reply">
                <span class="entry-seal" aria-hidden="true"></span>
                <p>{c.adminReply}</p>
              </div>
            {/if}
          </li>
        {/each}
      </ol>
    </section>
  {/if}
{/snippet}

<!-- Конец книги: две сургучные печати и запись для автора. Стоит сразу за текстом;
     на новую страницу уходит только если не поместилась. -->
{#snippet colophon()}
  <section class="colo">
    <p class="colo-orn" aria-hidden="true">❦</p>
    <div class="seals">
      <button type="button" class="seal hit" class:seal--on={myVote === 1} aria-pressed={myVote === 1} onclick={like}>
        {#key stamped}<span class="wax" class:wax--stamp={stamped > 0}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 20.5s-7-4.3-9-8.6C1.7 8.7 3.600 5.300 6.800 5.300c1.900 0 3.500 1 5.200 3 1.700-2 3.300-3 5.200-3 3.200 0 5.100 3.400 3.800 6.600-2 4.300-9 8.600-9 8.600z"/></svg>
        </span>{/key}
        <span class="seal-cap">{$t('talesLikeAction')}{likes > 0 ? ` · ${likes}` : ''}</span>
      </button>
      <button type="button" class="seal hit" class:seal--on={copied} onclick={share}>
        <span class="wax">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17L17 7M9 7h8v8"/></svg>
        </span>
        <span class="seal-cap">{copied ? $t('talesCardCopied') : $t('talesCardShare')}</span>
      </button>
    </div>
    {#if needLogin}
      <p class="colo-login" role="status">{$t('talesVoteNeedLogin')} <a href="/login">{$t('talesVoteLogin')}</a></p>
    {/if}
    {#if noteSent}
      <p class="colo-sent" role="status">✦ {$t('commentsSent')}</p>
    {:else}
      <form class="note" onsubmit={writeNote} novalidate>
        <textarea class="note-body hit" rows="4" maxlength="1000" aria-label={$t('commentsBodyLabel')} bind:value={note} oninput={saveDraft}></textarea>
        <div class="note-foot">
          {#if !authStore.isLoggedIn}
            <input class="note-name hit" type="text" maxlength="100" aria-label={$t('commentsNameLabel')} placeholder={$t('commentsNameLabel')} bind:value={noteName} oninput={saveDraft} />
          {/if}
          <button type="submit" class="note-send hit" disabled={noteSending || !note.trim()}>
            {noteSending ? $t('commentsSubmitting') : $t('commentsSubmit')}
          </button>
        </div>
        {#if noteError}<p class="note-err" role="alert">{noteError}</p>{/if}
      </form>
    {/if}
  </section>
{/snippet}

{#snippet figure(frame: Frame)}
  <figure class="art" style="--ratio: {scene?.ratio ?? '3 / 4'}">
    <img class="art-img" src={frame.url} alt="" loading="eager" decoding="sync" draggable="false" />
  </figure>
{/snippet}

{#snippet pastedown()}
  <div class="paste">
    {#snippet pastePhoto()}
      {#if photo}
        <AppImage src={photo} alt="" class="paste-img" sizes="420px" loading="eager" />
      {:else}
        <span class="paste-empty">✦</span>
      {/if}
    {/snippet}
    {#if workHref}
      <a
        class="paste-photo paste-photo--link"
        href={workHref}
        tabindex="-1"
        title={figurineNameIn(tale, $lang)}
        onclick={() => interact('tale_book_work', tale.slug, null, false)}
      >{@render pastePhoto()}</a>
    {:else}
      <div class="paste-photo">{@render pastePhoto()}</div>
    {/if}
    <p class="paste-name">{figurineNameIn(tale, $lang)}</p>
    <p class="paste-mark" aria-hidden="true">✦ ✦ ✦</p>
  </div>
{/snippet}

<!-- Страница целиком: бумага, поле набора и номер. `side` решает, у какого края тень переплёта. -->
{#snippet pageBox(p: number, side: 'l' | 'r' | 's', face: '' | 'front' | 'back', hidden: boolean)}
  <div class="pb pb--{side}" class:pb--front={face === 'front'} class:pb--back={face === 'back'}>
    {#if p >= 0 && p < totalPages}
      <div class="pbody" aria-hidden={hidden ? 'true' : undefined}>
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
<!-- Арка карточек-ссылок; книгу открывают и там, где обтравки ещё нет. -->
<ArchClip />
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
        <div class="shadow shadow--r" aria-hidden="true"></div>
        <div class="shadow shadow--l" aria-hidden="true"></div>
        <div class="boards boards--r" aria-hidden="true"></div>
        <div class="boards boards--l" aria-hidden="true"></div>
        <div class="sheet" class:sheet--hit={overHit} onpointermove={probe} onpointerdown={probe} onpointerleave={() => (overHit = false)}>
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

          <button
            type="button"
            class="edge edge--prev"
            aria-label={$t('talesBookPrev')}
            disabled={view === 0 || !swung || !!turn || closing}
            onclick={(e) => pageClick(e, -1)}
          ><span aria-hidden="true">‹</span></button>
          <button
            type="button"
            class="edge edge--next"
            aria-label={$t('talesBookNext')}
            disabled={atEnd || !swung || !!turn || closing}
            onclick={(e) => pageClick(e, 1)}
          ><span aria-hidden="true">›</span></button>
          {#if guestPage > 0 && Math.floor(guestPage / cols) > view && !turn && swung}
            <button type="button" class="ribbon" aria-label={$t('commentsTitle')} title={$t('commentsTitle')} onclick={toGuests}>
              <span aria-hidden="true">{guests.length}</span>
            </button>
          {/if}
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
        {#if views <= 10}
          {#each Array(views) as _, i}<i class:on={i === view}></i>{/each}
        {:else}
          <b class="count">{view + 1} / {views}</b>
        {/if}
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
  }
  /* Затемнение и размытие — отдельный слой, который только проявляется:
     анимация самого `backdrop-filter` в Safari мерцает рядом с 3D-листом. */
  .veil::before {
    content: '';
    position: absolute;
    inset: 0;
    background: rgba(20, 13, 10, 0.82);
    -webkit-backdrop-filter: blur(5px);
    backdrop-filter: blur(5px);
    opacity: 0;
    transition: opacity 0.6s ease;
  }
  .veil.on::before { opacity: 1; }

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
    will-change: transform;
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
  }
  /* Левая половина проявляется, когда крышка уже перешла за корешок: раньше
     она стояла с первого кадра, и слева был пустой коричневый переплёт, а
     крышка ещё лежала справа. При закрытии — зеркало. Левая половина — это
     отдельные слои (`boards--l`, `shadow--l`, левая страница), которые меняют
     только прозрачность: анимированный `clip-path` по линии корешка в Safari
     расходился с 3D-листом, и стык дёргался. */
  .boards,
  .shadow {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 50%;
  }
  .boards--r,
  .shadow--r { left: 50%; border-radius: 0 9px 9px 0; }
  .boards--l,
  .shadow--l { left: 0; border-radius: 4px 0 0 4px; }
  .boards--l,
  .shadow--l,
  .pb--l {
    opacity: 0;
    transition: opacity 0.3s ease;
  }
  .book.opened .boards--l,
  .book.opened .shadow--l,
  .book.opened .pb--l {
    opacity: 1;
    transition: opacity 0.4s ease 0.55s;
  }
  /* Тень книги: закрытая — только под правой половиной, раскрытая — под обеими.
     Лежит под переплётом, поэтому шов между половинами её не пересекает. */
  .shadow { box-shadow: 0 30px 40px rgba(0, 0, 0, 0.55); }
  .boards {
    background:
      radial-gradient(ellipse 100% 90% at 50% 50%, transparent 50%, rgba(0, 0, 0, 0.4) 100%),
      linear-gradient(160deg, #4d2a22, #2f1914);
    background-size: 200% 100%, 200% 100%;
  }
  /* Окантовка только по внешнему краю; половины перекрываются на пиксель,
     иначе по корешку виден шов. */
  .boards--r {
    background-position: 100% 0, 100% 0;
    box-shadow: inset -1px 0 0 rgba(217, 191, 134, 0.22), inset 0 1px 0 rgba(217, 191, 134, 0.22), inset 0 -1px 0 rgba(217, 191, 134, 0.22);
  }
  .boards--l {
    width: calc(50% + 1px);
    background-position: 0 0, 0 0;
    box-shadow: inset 1px 0 0 rgba(217, 191, 134, 0.22), inset 0 1px 0 rgba(217, 191, 134, 0.22), inset 0 -1px 0 rgba(217, 191, 134, 0.22);
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
  .flow :global(.tipped) { break-inside: avoid; }
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
  .flow :global(.art) {
    /* Рамка в пропорциях сцены, по высоте не больше половины страницы; размер задан, чтобы разбивка не ждала загрузки. */
    height: calc(var(--ph) * 0.5);
    aspect-ratio: var(--ratio);
    max-width: 100%;
    margin: 0.4em auto 1em;
    padding: 4px;
    box-sizing: border-box;
    break-inside: avoid;
    background: linear-gradient(160deg, #e7d19a, #a88445 55%, #d9bf86);
    box-shadow: 0 2px 6px rgba(52, 37, 28, 0.3);
  }
  .flow :global(.art-img) {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover !important;
    object-position: 50% 40% !important;
    background: #120c09;
  }
  .paste-photo--link { cursor: pointer; }
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

  .the-end {
    margin: 1.4em 0 0;
    text-align: center;
    break-inside: avoid;
    font: italic 400 16px/1.4 'Cormorant Garamond', Georgia, serif;
    color: var(--muted, #5f4636);
  }

  /* ── Последняя страница ───────────────────────────────────────────────── */
  .colo {
    break-inside: avoid;
    padding-top: 1.6em;
    text-align: center;
  }
  .colo-orn {
    margin: 0 0 1.1em;
    font-size: 26px;
    line-height: 1;
    color: var(--copper, #c65f3c);
  }
  .seals { display: flex; justify-content: center; gap: clamp(14px, 3vw, 30px); }
  .seal {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: 96px;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    color: var(--muted, #5f4636);
    -webkit-tap-highlight-color: transparent;
  }
  .seal-cap {
    font: italic 400 15px/1.2 'Cormorant Garamond', Georgia, serif;
    text-wrap: balance;
  }
  .seal--on .seal-cap { color: var(--deep, #6f3b24); }
  /* Сургучная печать: неровный край, вдавленное кольцо, знак выдавлен в воске. */
  .wax {
    position: relative;
    display: grid;
    place-items: center;
    width: 64px;
    height: 62px;
    border-radius: 52% 48% 55% 45% / 47% 54% 46% 53%;
    background:
      radial-gradient(circle at 32% 26%, rgba(255, 200, 170, 0.5), transparent 38%),
      radial-gradient(circle at 60% 70%, #7d2a17, #a4452c 60%, #8a321d);
    box-shadow:
      0 2px 3px rgba(52, 37, 28, 0.45),
      inset 0 -3px 5px rgba(60, 15, 5, 0.5),
      inset 0 2px 3px rgba(255, 190, 160, 0.35);
    filter: saturate(0.82);
    transition: transform 0.18s ease, filter 0.25s;
  }
  .wax::before {
    content: '';
    position: absolute;
    inset: 9px;
    border-radius: 50%;
    border: 1.5px solid rgba(60, 15, 5, 0.38);
    box-shadow: 0 1px 0 rgba(255, 190, 160, 0.25);
  }
  .wax svg {
    position: relative;
    width: 26px;
    height: 26px;
    fill: none;
    stroke: rgba(60, 15, 5, 0.62);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    filter: drop-shadow(0 1px 0 rgba(255, 190, 160, 0.4));
  }
  .seal:hover .wax,
  .seal:focus-visible .wax { transform: translateY(-2px) rotate(-3deg); filter: saturate(1); }
  .seal:active .wax { transform: scale(0.93); }
  .seal--on .wax { filter: saturate(1.1); }
  .seal--on:first-child .wax svg { fill: rgba(60, 15, 5, 0.55); }
  .wax--stamp { animation: wax-stamp 0.55s cubic-bezier(0.2, 0.8, 0.3, 1); }
  @keyframes wax-stamp {
    0% { transform: scale(1.45); opacity: 0.2; }
    55% { transform: scale(0.9); opacity: 1; }
    100% { transform: scale(1); }
  }
  .colo-login,
  .colo-sent,
  .note-err {
    margin: 1.1em auto 0;
    max-width: 30em;
    font: italic 400 16px/1.4 'Cormorant Garamond', Georgia, serif;
    color: var(--muted, #5f4636);
  }
  .colo-login a { color: var(--deep, #6f3b24); }
  .colo-sent { color: var(--deep, #6f3b24); }
  .note-err { color: #8a2a1a; }
  /* Запись от руки: линованная бумага, чернила, подпись на черте. */
  .note { max-width: 31em; margin: 1.3em auto 0; text-align: left; }
  .note-body {
    display: block;
    width: 100%;
    box-sizing: border-box;
    resize: none;
    padding: 4px 6px;
    border: 0;
    border-bottom: 1px solid rgba(111, 59, 36, 0.35);
    background: repeating-linear-gradient(transparent 0 27px, rgba(111, 59, 36, 0.22) 27px 28px);
    font: italic 400 19px/28px 'Cormorant Garamond', Georgia, serif;
    color: var(--ink, #34251c);
    outline: none;
  }
  .note-body:focus { background-color: rgba(255, 252, 246, 0.5); }
  .note-foot { display: flex; align-items: flex-end; justify-content: space-between; gap: 14px; margin-top: 12px; }
  .note-name {
    flex: 1;
    min-width: 0;
    padding: 4px 2px;
    border: 0;
    border-bottom: 1px solid rgba(111, 59, 36, 0.45);
    background: transparent;
    font: italic 400 17px/1.3 'Cormorant Garamond', Georgia, serif;
    color: var(--ink, #34251c);
    outline: none;
  }
  .note-send {
    margin-left: auto;
    padding: 8px 18px;
    border: 1px solid rgba(111, 59, 36, 0.55);
    border-radius: 2px;
    background: rgba(255, 252, 246, 0.6);
    font: 500 12px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--deep, #6f3b24);
    cursor: pointer;
  }
  .note-send:disabled { opacity: 0.45; cursor: default; }
  /* Книга отзывов. */
  .guests { break-before: column; padding-top: 0.4em; text-align: center; }
  .guests-title {
    margin: 0 0 1em;
    font: italic 400 24px/1.2 'Cormorant Garamond', Georgia, serif;
    color: var(--deep, #6f3b24);
  }
  .entries { list-style: none; margin: 0; padding: 0; text-align: left; }
  .entry { break-inside: avoid; margin: 0 0 1.5em; }
  .entry-body {
    margin: 0;
    font: italic 400 18.5px/1.55 'Cormorant Garamond', Georgia, serif;
    color: #2f2a4a;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .entry-sign {
    margin: 0.35em 0 0;
    text-align: right;
    font: italic 500 16px/1.3 'Cormorant Garamond', Georgia, serif;
    color: var(--muted, #5f4636);
  }
  .entry-date { font-weight: 400; opacity: 0.8; }
  /* Ответ автора — другими чернилами, с маленькой печатью. */
  .entry-reply {
    position: relative;
    margin: 0.7em 0 0 1.6em;
    padding: 0.1em 0 0 1.5em;
    border-left: 1px solid rgba(111, 59, 36, 0.3);
  }
  .entry-reply p {
    margin: 0;
    font: italic 400 18px/1.5 'Cormorant Garamond', Georgia, serif;
    color: #7a2b18;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .entry-seal {
    position: absolute;
    left: -9px;
    top: 0.5em;
    width: 16px;
    height: 15px;
    border-radius: 52% 48% 55% 45% / 47% 54% 46% 53%;
    background: radial-gradient(circle at 32% 28%, rgba(255, 200, 170, 0.5), transparent 40%), #9a3a22;
    box-shadow: 0 1px 2px rgba(52, 37, 28, 0.45), inset 0 -1px 2px rgba(60, 15, 5, 0.5);
  }

  /* Закладка-ленточка: свисает с верхнего обреза правой страницы, ведёт к отзывам. */
  .ribbon {
    position: absolute;
    top: 0;
    /* Висит в поле страницы, а не над текстом: поле шириной `--m`, лента по его середине. */
    right: calc(var(--m) / 2 - 11px);
    z-index: 8;
    width: 22px;
    height: 84px;
    padding: 12px 0 0;
    border: 0;
    cursor: pointer;
    color: rgba(255, 238, 220, 0.92);
    font: italic 500 13px/1 'Cormorant Garamond', Georgia, serif;
    background: linear-gradient(90deg, #7d2a17, #a4452c 45%, #8a321d);
    clip-path: polygon(0 0, 100% 0, 100% 100%, 50% 86%, 0 100%);
    transform-origin: top center;
    animation: ribbon-sway 4.5s ease-in-out infinite;
    -webkit-tap-highlight-color: transparent;
  }
  .ribbon:hover { height: 94px; }
  @keyframes ribbon-sway { 0%, 100% { transform: rotate(-1.2deg); } 50% { transform: rotate(1.2deg); } }
  @media (prefers-reduced-motion: reduce) { .ribbon { animation: none; } }

  .sheet--hit .edge { pointer-events: none; }

  /* Нажатие листает с любого места половины книги: левая половина — назад,
     правая — вперёд. Знак и тень видны у обреза, пока указатель над половиной. */
  .edge {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 6;
    width: 50%;
    display: flex;
    align-items: center;
    padding: 0 10px;
    box-sizing: border-box;
    border: 0;
    background: transparent;
    color: var(--muted, #5f4636);
    font: 300 40px/1 'Cormorant Garamond', Georgia, serif;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.25s;
    -webkit-tap-highlight-color: transparent;
  }
  .edge--prev {
    left: 0;
    justify-content: flex-start;
    background: linear-gradient(90deg, rgba(52, 37, 28, 0.1), transparent) no-repeat left / 18% 100%;
  }
  .edge--next {
    right: 0;
    justify-content: flex-end;
    background: linear-gradient(270deg, rgba(52, 37, 28, 0.1), transparent) no-repeat right / 18% 100%;
  }
  .edge:hover:not(:disabled),
  .edge:focus-visible:not(:disabled) { opacity: 1; }
  .edge:disabled { cursor: default; pointer-events: none; }

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
    will-change: transform;
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
  /* Матовые таблички обложки (`backdrop-filter`) внутри вращающегося листа в
     Safari мерцают: размытие пересчитывается на каждом кадре поворота. У
     откидываемой обложки табличка — тот же полупрозрачный тон без размытия. */
  .leaf-front :global(.lab) {
    -webkit-backdrop-filter: none !important;
    backdrop-filter: none !important;
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
  .count { font: 600 11px/1 Inter, system-ui, sans-serif; letter-spacing: 0.14em; color: #e4cd93; }

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
    .block { opacity: 0; transition: opacity 0.5s ease; }
    .book.opened .block { opacity: 1; transition: opacity 0.5s ease; }
    .boards--r,
    .shadow--r { left: 0; width: 100%; border-radius: 4px 9px 9px 4px; }
    .boards--r {
      background-size: auto;
      background-position: 0 0;
      box-shadow: inset 0 0 0 1px rgba(217, 191, 134, 0.22);
    }
    .boards--l,
    .shadow--l { display: none; }
    .leaf { display: none; }
    .pb--l { opacity: 1 !important; }
  }

  @media (prefers-reduced-motion: reduce) {
    .book,
    .block,
    .boards,
    .shadow,
    .pb,
    .veil::before,
    .leaf,
    .stage,
    .bar,
    .veil { transition: none; }
  }
</style>
