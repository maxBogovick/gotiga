<script lang="ts" module>
  /** Событие, по которому шапка просит открыть поиск (клавиша «/»). */
  export const SITE_SEARCH_OPEN = 'site-search:open';
</script>

<script lang="ts">
  import { figurineNameIn } from '$lib/workText';
  // Поиск в шапке. Два вида одного и того же:
  //   · `field` — строка с лупой в верхней строке шапки на компьютере; лист
  //     выдачи раскрывается из-под неё;
  //   · `button` — одна лупа (шапка свёрнута прокруткой или это телефон);
  //     нажатие открывает тот же лист со своим полем вверху, на телефоне —
  //     во весь экран.
  // Лист один: сверху отбор печатями (те же, что разделы шапки), ниже работы в
  // арках, небылицы и вестник строками, карты в рамке карты. Под каждой вещью
  // сказано словами, почему она найдена.
  import { onMount, tick, untrack, type Component } from 'svelte';
  import { page } from '$app/state';
  import { afterNavigate, goto } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import { figurineHref } from '$lib/figurineHref';
  import { leafCopy, leafHref, quietDate, leafWhen, leafCoverUrl } from '$lib/gazette';
  import { cardHref } from '$lib/world';
  import { api, resolveMediaUrl } from '$lib/api';
  import { indexIn, loadIndex } from '$lib/site-index';
  import { buildVocabulary, suggest } from '$lib/site-suggest';
  import {
    ASK_TOPICS, TOPIC_LABEL, forPage, groupByTopic, inTopic, matchQuestions, pageKeyOf, topicsWithQuestions,
    type AskTopic,
  } from '$lib/askQuestions';
  import { interact } from '$lib/analytics';
  import { voiceSupported, startVoice, type Voice } from '$lib/voiceInput';
  import { readingMinutes, minutesKey } from '$lib/tales';
  import {
    searchSite,
    rankFound,
    foundTotal,
    markParts,
    shortWorkName,
    cardTitle,
    type FoundWhy,
    type SearchScope,
    type SiteFound,
    type SiteIndex,
  } from '$lib/site-search';
  import SectionIcon, { type SectionIconName } from '$lib/components/SectionIcon.svelte';
  import { WAX_PATH } from '$lib/components/SectionSeal.svelte';
  import type { AskQuestion, BattleCard, FigurineListItem, SiteHit } from '$lib/types/api';

  let { variant = 'field' }: { variant?: 'field' | 'button' } = $props();

  const uid = Math.random().toString(36).slice(2, 8);
  const listId = `site-search-list-${uid}`;
  const archId = `site-search-arch-${uid}`;

  let indexRaw = $state.raw<SiteIndex | null>(null);
  // Названия работ на языке читателя (`indexIn`).
  let index = $derived(indexRaw ? indexIn(indexRaw, $lang) : null);
  let loadFailed = $state(false);
  let query = $state('');
  let open = $state(false);
  let scope = $state<SearchScope>('all');
  let active = $state(-1);
  // Порядок от сервера (`/search/site`): все виды вещей одним списком. Пока он не
  // пришёл, показывается то, что нашлось по словам в самом браузере.
  let ranked = $state.raw<SiteHit[] | null>(null);
  let rankedFor = '';
  let root = $state<HTMLElement | null>(null);
  let input = $state<HTMLInputElement | null>(null);
  let panel = $state<HTMLElement | null>(null);
  let trigger = $state<HTMLButtonElement | null>(null);
  let reduceMotion = $state(false);

  // ── Вопросы ─────────────────────────────────────────────────────────────
  // Код ответов (разбор, каталог, хранилище разговора) нужен только тому, кто
  // открыл поиск, поэтому грузится при первом открытии, а не на каждой странице.
  interface AskKit {
    siteAsk: typeof import('$lib/stores/site-ask.svelte').siteAsk;
    QUESTION_MAX: number;
    askShape: typeof import('$lib/site-ask').askShape;
    hasPageContext: typeof import('$lib/site-ask-page').hasPageContext;
  }
  let kit = $state.raw<AskKit | null>(null);
  let Talk = $state.raw<Component<{ onfollow: () => void; compact?: boolean }> | null>(null);
  function ensureAsk() {
    if (kit) return;
    void Promise.all([
      import('$lib/stores/site-ask.svelte'),
      import('$lib/site-ask'),
      import('$lib/site-ask-page'),
      import('$lib/components/SiteSearchTalk.svelte'),
    ]).then(([store, ask, pageCtx, talk]) => {
      kit = {
        siteAsk: store.siteAsk,
        QUESTION_MAX: store.QUESTION_MAX,
        askShape: ask.askShape,
        hasPageContext: pageCtx.hasPageContext,
      };
      Talk = talk.default;
    }).catch(() => {});
  }

  // Список вопросов каталога: тема, страницы и формулировки каждого. Не пришёл —
  // окно показывает прежние четыре вопроса, а подсказок при печати нет.
  let askList = $state.raw<AskQuestion[]>([]);
  $effect(() => {
    const l = $lang;
    if (!kit) return;
    void api.askQuestions(l).then((list) => {
      if (l === untrack(() => $lang)) askList = list;
    });
  });

  /** Слово для поиска, вопрос или команда. До загрузки кода ответов — всегда слово. */
  let shape = $derived(kit ? kit.askShape(query) : 'search');

  // Ответ «на лету»: пока человек печатает вопрос, сайт показывает ответ над
  // результатами. Спрашивается он с паузой, одним словом не бывает, а команду
  // («открой игру») не выполняет, пока не нажат Enter.
  const LIVE_MS = 450;
  let liveTimer: ReturnType<typeof setTimeout> | undefined;
  let liveFor = '';
  $effect(() => {
    const q = query.trim();
    const k = kit;
    const sh = shape;
    clearTimeout(liveTimer);
    if (!k) return;
    if (open && sh === 'question' && q.length >= 5) {
      if (q === liveFor) return;
      liveTimer = setTimeout(() => {
        liveFor = q;
        void k.siteAsk.ask(q, 'live');
      }, LIVE_MS);
    } else {
      liveFor = '';
      untrack(() => k.siteAsk.dropLive());
    }
    return () => clearTimeout(liveTimer);
  });

  /** Отправить вопрос: ответ, уже показанный на лету, становится обычным. */
  function sendQuestion(text: string, via: 'input' | 'chip' | 'voice') {
    if (!kit) return;
    const q = text.trim().slice(0, kit.QUESTION_MAX);
    if (!q) return;
    clearTimeout(liveTimer);
    const live = kit.siteAsk.turns.find((x) => x.live && x.question === q);
    if (live) kit.siteAsk.commitLive();
    else void kit.siteAsk.ask(q, via);
    liveFor = '';
    query = '';
    active = -1;
    input?.focus();
  }

  // ── Голос ────────────────────────────────────────────────────────────────
  let canVoice = $state(false);
  let listening = $state(false);
  let voiceNote = $state('');
  let voice: Voice | null = null;
  let heard = '';
  let noteTimer: ReturnType<typeof setTimeout> | undefined;

  function note(key: 'askVoiceDenied' | 'askVoiceFailed' | 'askVoiceSilence') {
    voiceNote = $t(key);
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => (voiceNote = ''), 5000);
  }

  function toggleVoice() {
    if (listening) {
      voice?.stop();
      return;
    }
    voiceNote = '';
    heard = '';
    interact('site_ask_voice', 'start');
    voice = startVoice($lang, {
      onText: (spoken, final) => {
        query = spoken;
        if (final) heard = spoken;
      },
      onEnd: () => {
        listening = false;
        voice = null;
        // Фраза дослушана — вопрос уходит сам: ради этого голос и включали.
        // Остановили вручную раньше — набранное остаётся в поле, его можно поправить.
        if (heard) {
          const q = heard;
          heard = '';
          sendQuestion(q, 'voice');
        }
      },
      onError: (e) => note(e === 'denied' ? 'askVoiceDenied' : e === 'silence' ? 'askVoiceSilence' : 'askVoiceFailed'),
    });
    listening = !!voice;
    if (listening) query = '';
  }

  let local = $derived<SiteFound | null>(index ? searchSite(index, query, $lang, null) : null);
  let found = $derived<SiteFound | null>(
    index && ranked && rankedFor === query.trim() ? rankFound(index, ranked, local) : local,
  );
  let asked = $derived(query.trim().length >= 2);

  // Подсказки при вводе: хвост слова серым и ряд вариантов под полем.
  let vocab = $derived(index ? buildVocabulary(index, $lang) : []);
  let sugg = $derived(suggest(vocab, query));
  /** Хвост виден, только пока курсор в конце строки: иначе он лёг бы поверх набранного. */
  let caretAtEnd = $state(true);
  function syncCaret() {
    caretAtEnd = !!input && input.selectionStart === query.length && input.selectionEnd === query.length;
  }
  function acceptGhost(): boolean {
    if (!sugg.ghost || !caretAtEnd) return false;
    query += sugg.ghost;
    interact('site_suggest', 'accept');
    return true;
  }

  // ── Подсказка в пустом поле: пример выбирается один раз за заход ────────
  // Прежде он сменялся каждые 3,2 с без возможности остановить: движение в
  // постоянно видимой части страницы дольше пяти секунд читателю не отключить
  // (WCAG 2.2.2). Разнообразие осталось — от захода к заходу, а не на глазах.
  let hints = $derived($t('searchHints').split('|'));
  let hintAt = $state(0);

  // ── Данные ──────────────────────────────────────────────────────────────
  function ensureIndex() {
    if (indexRaw) return;
    loadIndex()
      .then((i) => { indexRaw = i; loadFailed = false; })
      .catch(() => { loadFailed = true; });
  }

  // Поиск по всему сайту на сервере — с паузой после ввода: он считает дольше
  // слов, а ручка ограничена тридцатью запросами в минуту.
  let rankedTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const q = query.trim();
    clearTimeout(rankedTimer);
    if (q.length < 2) {
      ranked = null;
      rankedFor = '';
      return;
    }
    if (q === rankedFor) return;
    rankedTimer = setTimeout(() => {
      api.siteSearch(q, 40).then((hits) => {
        // Пока считалось, запрос мог измениться: чужой порядок не берётся.
        if (hits && q === query.trim()) {
          ranked = hits;
          rankedFor = q;
        }
      });
    }, 380);
    return () => clearTimeout(rankedTimer);
  });

  // ── Открыть и закрыть ───────────────────────────────────────────────────
  async function openPanel(focus = true) {
    ensureIndex();
    ensureAsk();
    if (!open) {
      interact('site_search', variant === 'field' ? 'field' : 'button');
      startMode = 'home';
    }
    open = true;
    active = -1;
    if (focus) {
      await tick();
      input?.focus();
    }
  }

  function closePanel(refocus = false) {
    if (!open) return;
    // Закрыли лист, не стерев вопрос: ответ, который был на виду, считается заданным.
    kit?.siteAsk.commitLive();
    voice?.stop();
    open = false;
    active = -1;
    if (variant === 'field') input?.blur();
    else if (refocus) trigger?.focus();
  }

  function onOutside(e: PointerEvent) {
    if (!open || !root) return;
    if (root.contains(e.target as Node) || panel?.contains(e.target as Node)) return;
    closePanel();
  }

  // Шапка просит открыть (клавиша «/»). Откликается тот вид, который сейчас виден.
  function onOpenRequest() {
    if (!root || root.getClientRects().length === 0) return;
    const visible = variant === 'field' ? input : trigger;
    if (!visible || visible.getClientRects().length === 0) return;
    void openPanel();
  }

  afterNavigate(() => {
    open = false;
    query = '';
    scope = 'all';
  });

  // ── Выдача ──────────────────────────────────────────────────────────────
  type Row =
    | { kind: 'work'; href: string; item: FigurineListItem; why: FoundWhy }
    | { kind: 'tale' | 'leaf'; href: string; item: import('$lib/types/api').GazetteLeaf; why: FoundWhy }
    | { kind: 'card'; href: string; item: BattleCard; why: FoundWhy }
    | { kind: 'ask'; href: string; text: string };

  const CAP = { works: 6, tales: 5, gazette: 3, cards: 6 } as const;
  const CAP_ALONE = { works: 24, tales: 12, gazette: 12, cards: 24 } as const;
  const SRC = 'header_search';

  function cap(k: keyof typeof CAP) {
    return scope === k ? CAP_ALONE[k] : CAP[k];
  }
  function shows(k: Exclude<SearchScope, 'all'>) {
    return scope === 'all' || scope === k;
  }

  let worksShown = $derived(found && shows('works') ? found.works.slice(0, cap('works')) : []);
  let talesShown = $derived(found && shows('tales') ? found.tales.slice(0, cap('tales')) : []);
  let gazetteShown = $derived(found && shows('gazette') ? found.gazette.slice(0, cap('gazette')) : []);
  let cardsShown = $derived(found && shows('cards') ? found.cards.slice(0, cap('cards')) : []);

  // Пустое поле: несколько вопросов автору и самая свежая небылица. Входы в
  // разделы здесь не повторяются: они стоят в шапке над полем.
  let onPage = $derived(kit ? kit.hasPageContext({ path: page.url.pathname, card: page.url.searchParams.get('card') }) : false);
  // Что в пустом поле: первым делом вопросы про страницу, на которой человек стоит
  // (каталог отмечает их сам), затем темы и «все вопросы». Нет вопросов страницы
  // или список не пришёл — прежние четыре.
  let startMode = $state<'home' | 'all' | AskTopic>('home');
  let pageQuestions = $derived.by(() => {
    const here = pageKeyOf(page.url.pathname);
    return here ? forPage(askList, here, 3) : [];
  });
  let homeQuestions = $derived(
    pageQuestions.length
      ? [...(onPage ? [$t('askChipPage')] : []), ...pageQuestions.map((q) => q.question), $t('askChipMatch')].slice(0, 4)
      : onPage
        ? [$t('askChipPage'), $t('askChipOrder'), $t('askChipStart'), $t('askChipMatch')]
        : [$t('askChipOrder'), $t('askChipStart'), $t('askChipPlay'), $t('askChipMatch')],
  );
  let startTopics = $derived(topicsWithQuestions(askList));
  let allGroups = $derived(groupByTopic(askList));
  let questions = $derived(
    startMode === 'home'
      ? homeQuestions
      : startMode === 'all'
        ? allGroups.flatMap((g) => g.items.map((q) => q.question))
        : inTopic(askList, startMode).map((q) => q.question),
  );
  // Пока человек печатает, подсказываются вопросы каталога, в которых есть его слова.
  let matched = $derived(asked && scope === 'all' && kit ? matchQuestions(askList, query, 3) : []);

  function openTopic(mode: 'all' | AskTopic) {
    interact('site_ask_topic', mode);
    startMode = mode;
    active = -1;
    input?.focus();
  }
  let freshTale = $derived(
    index ? ([...index.tales].sort((a, b) => leafWhen(b).localeCompare(leafWhen(a)))[0] ?? null) : null,
  );

  // Плоский список того, что выбирают стрелками, в порядке листа.
  let rows = $derived<Row[]>(
    asked
      ? [
          ...matched.map((q) => ({ kind: 'ask' as const, href: '', text: q.question })),
          ...worksShown.map((f) => ({ kind: 'work' as const, href: figurineHref(f.item, SRC), item: f.item, why: f.why })),
          ...talesShown.map((f) => ({ kind: 'tale' as const, href: leafHref(f.item, SRC), item: f.item, why: f.why })),
          ...gazetteShown.map((f) => ({ kind: 'leaf' as const, href: leafHref(f.item, SRC), item: f.item, why: f.why })),
          ...cardsShown.map((f) => ({ kind: 'card' as const, href: cardHref(f.item), item: f.item, why: f.why })),
        ]
      : [
          ...(kit ? questions.map((text) => ({ kind: 'ask' as const, href: '', text })) : []),
          ...(freshTale && startMode === 'home' ? [{ kind: 'tale' as const, href: leafHref(freshTale, SRC), item: freshTale, why: { kind: 'name' } as FoundWhy }] : []),
        ],
  );

  let workBase = $derived(matched.length);
  let taleBase = $derived(workBase + worksShown.length);
  let leafBase = $derived(taleBase + talesShown.length);
  let cardBase = $derived(leafBase + gazetteShown.length);

  $effect(() => {
    void query;
    void scope;
    active = -1;
  });

  function whyText(why: FoundWhy): string {
    switch (why.kind) {
      case 'name': return $t('searchWhyName');
      case 'card': return $t('searchWhyCard').replace('{x}', why.title);
      case 'race': return $t('searchWhyRace').replace('{x}', why.race);
      case 'craft': return $t('searchWhyCraft');
      case 'meaning': return $t('searchWhyMeaning');
      case 'work': return $t('searchWhyWork');
      default: return $t('searchWhyText');
    }
  }

  function workImage(w: FigurineListItem): string | null {
    return resolveMediaUrl(w.thumbUrl ?? w.faceImageUrl);
  }

  // Лист лупы живёт в корне страницы (см. разметку); лист поля — под полем.
  function toBody(node: HTMLElement, on: boolean) {
    if (on) document.body.appendChild(node);
    return {
      destroy() {
        if (on) node.remove();
      },
    };
  }

  // ── Отбор печатями ──────────────────────────────────────────────────────
  const SCOPES: { key: SearchScope; icon: SectionIconName; label: string }[] = [
    { key: 'all', icon: 'more', label: 'searchAll' },
    { key: 'works', icon: 'works', label: 'dockWorks' },
    { key: 'tales', icon: 'tales', label: 'navTales' },
    { key: 'gazette', icon: 'gazette', label: 'navGazette' },
    { key: 'cards', icon: 'game', label: 'searchCards' },
  ];

  function scopeCount(k: SearchScope): number | null {
    if (found && asked) return foundTotal(found, k);
    if (!index) return null;
    if (k === 'all') return index.works.length + index.tales.length + index.gazette.length + index.cards.length;
    return index[k].length;
  }

  // ── Клавиши ─────────────────────────────────────────────────────────────
  function setActive(n: number) {
    if (rows.length === 0) return;
    active = (n + rows.length) % rows.length;
    void tick().then(() => {
      panel?.querySelector(`[data-row="${active}"]`)?.scrollIntoView({ block: 'nearest' });
    });
  }

  function choose(row: Row) {
    if (row.kind === 'ask') {
      interact('site_search_open', 'ask:question');
      sendQuestion(row.text, 'chip');
      return;
    }
    interact('site_search_open', `${row.kind}:${scope}`);
    // Выбрали вещь из списка: ответ на лету не задавали, он не считается.
    kit?.siteAsk.dropLive();
    closePanel();
    void goto(row.href);
  }

  function onKey(e: KeyboardEvent) {
    // Серый хвост принимают → и Tab. Tab занят только пока хвост виден: без
    // него он уходит из поля, как всегда, и остановка в шапке остаётся одна.
    if ((e.key === 'ArrowRight' || e.key === 'Tab') && active < 0 && !e.shiftKey && acceptGhost()) {
      e.preventDefault();
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (!open) void openPanel(false);
      setActive(active + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive(active - 1);
    } else if ((e.key === 'ArrowRight' || e.key === 'ArrowLeft') && active >= 0) {
      e.preventDefault();
      setActive(active + (e.key === 'ArrowRight' ? 1 : -1));
    } else if (e.key === 'Enter') {
      if (active >= 0 && rows[active]) {
        e.preventDefault();
        choose(rows[active]);
      } else if (query.trim() && shape !== 'search') {
        e.preventDefault();
        sendQuestion(query, 'input');
      } else if (asked) {
        e.preventDefault();
        closePanel();
        void goto(`/figurines?q=${encodeURIComponent(query.trim())}`);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (query) query = '';
      else if (startMode !== 'home') startMode = 'home';
      else closePanel(true);
    }
  }

  function useExample(word: string) {
    query = word;
    input?.focus();
  }

  onMount(() => {
    reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    canVoice = voiceSupported();
    document.addEventListener('pointerdown', onOutside, { capture: true });
    window.addEventListener(SITE_SEARCH_OPEN, onOpenRequest);
    if (variant === 'field') hintAt = Math.floor(Math.random() * Math.max(1, hints.length));
    return () => {
      document.removeEventListener('pointerdown', onOutside, { capture: true });
      window.removeEventListener(SITE_SEARCH_OPEN, onOpenRequest);
      voice?.stop();
      clearTimeout(noteTimer);
    };
  });

  // На телефоне лист во весь экран: страница под ним не прокручивается.
  $effect(() => {
    if (!open || variant !== 'button' || typeof window === 'undefined') return;
    if (!window.matchMedia('(max-width: 1024px)').matches) return;
    const prev = document.documentElement.style.overflow;
    document.documentElement.style.overflow = 'hidden';
    return () => { document.documentElement.style.overflow = prev; };
  });

  let archiveHref = $derived(asked ? `/figurines?q=${encodeURIComponent(query.trim())}` : '/figurines');
</script>

{#snippet inputRow(inPanel: boolean)}
  <div class="field" class:field--panel={inPanel}>
    <span class="loupe"><SectionIcon name="search" variant="line" size={22} /></span>
    <input
      bind:this={input}
      bind:value={query}
      class="input"
      type="search"
      autocomplete="off"
      spellcheck="false"
      role="combobox"
      aria-expanded={open}
      aria-controls={listId}
      aria-autocomplete="list"
      aria-activedescendant={active >= 0 ? `${listId}-${active}` : undefined}
      aria-label={$t('searchLabel')}
      onfocus={() => void openPanel(false)}
      onpointerenter={() => { ensureIndex(); ensureAsk(); }}
      onkeydown={onKey}
      onkeyup={syncCaret}
      onclick={syncCaret}
      oninput={syncCaret}
    />
    {#if sugg.ghost && caretAtEnd}
      <span class="ghost" aria-hidden="true"><span class="ghost-typed">{query}</span>{sugg.ghost}<kbd class="ghost-key">Tab</kbd></span>
    {/if}
    {#if !query}
      <span class="hint" aria-hidden="true">
        <span class="hint-lead">{$t('searchLead')}</span>
        <span class="hint-word">{hints[hintAt % hints.length]}</span>
      </span>
    {/if}
    {#if canVoice}
      <button
        type="button"
        class="mic"
        class:on={listening}
        aria-pressed={listening}
        aria-label={$t('askVoice')}
        title={$t('askVoiceTip')}
        onpointerdown={(e) => e.preventDefault()}
        onclick={toggleVoice}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <rect x="9" y="3" width="6" height="12" rx="3" fill="currentColor" />
          <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v3" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </button>
    {/if}
    {#if query}
      <button type="button" class="clear" aria-label={$t('searchClear')} onpointerdown={(e) => { e.preventDefault(); query = ''; input?.focus(); }}>×</button>
    {:else if !inPanel}
      <kbd class="key" aria-hidden="true">/</kbd>
    {/if}
    {#if inPanel}
      <button type="button" class="close" onclick={() => closePanel(true)}>{$t('dockClose')}</button>
    {/if}
  </div>
{/snippet}

{#snippet parts(text: string)}
  {#each markParts(text, found?.words ?? []) as p, i (i)}{#if p.hit}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}
{/snippet}

{#snippet workPlate(w: FigurineListItem, why: FoundWhy | null, at: number)}
  <a
    href={figurineHref(w, SRC)}
    class="wk"
    class:act={active === at}
    role="option"
    id="{listId}-{at}"
    aria-selected={active === at}
    data-row={at}
    style:--i={at}
    onclick={(e) => { e.preventDefault(); choose(rows[at]); }}
    onpointerenter={() => (active = at)}
  >
    <span class="ph">
      {#if workImage(w)}<img src={workImage(w)} alt="" loading="lazy" decoding="async" />{/if}
      <svg class="arch-line" viewBox="0 0 100 125" preserveAspectRatio="none" aria-hidden="true"><path d="M1 124V52C1 26 24 7 50 1c26 6 49 25 49 51v72z" /></svg>
      {#if w.status === 'sold'}
        <svg class="sold" viewBox="0 0 52 52" role="img" aria-label={$t('figurineStatusSold')}><path d={WAX_PATH} fill="#963a20" /><circle cx="26" cy="26" r="15" fill="none" stroke="#6f2512" opacity=".6" /></svg>
      {/if}
    </span>
    <span class="nm">{@render parts(shortWorkName(w.name))}</span>
    {#if why}<span class="why">{whyText(why)}</span>{/if}
  </a>
{/snippet}

<div class="ss ss--{variant}" class:on={open} bind:this={root}>
  {#if variant === 'field'}
    {@render inputRow(false)}
    <span class="ink" aria-hidden="true"></span>
  {:else}
    <button
      bind:this={trigger}
      type="button"
      class="trigger"
      aria-label={$t('searchLabel')}
      title={$t('searchLabel')}
      aria-expanded={open}
      onclick={() => (open ? closePanel() : void openPanel())}
      onpointerenter={() => { ensureIndex(); ensureAsk(); }}
    >
      <SectionIcon name="search" variant="line" size={21} />
    </button>
  {/if}

  {#if open}
    <!-- Лист лупы выносится в корень страницы: шапка размывает фон под собой
         и тем самым запирает «закреплённые» элементы в своей высоте. -->
    <div
      class="panel"
      class:panel--float={variant === 'button'}
      class:panel--quiet={reduceMotion}
      bind:this={panel}
      style:--arch="url(#{archId})"
      use:toBody={variant === 'button'}
    >
      <!-- Стрельчатая арка в долях коробки, своя: обтравка полки небылиц
           объявлена только на тех страницах, где полка стоит. -->
      <svg class="defs" width="0" height="0" aria-hidden="true">
        <clipPath id={archId} clipPathUnits="objectBoundingBox"><path d="M0 1V.42C0 .21.24.06.5 0c.26.06.5.21.5.42V1z" /></clipPath>
      </svg>
      {#if variant === 'button'}{@render inputRow(true)}{/if}

      {#if sugg.chips.length}
        <div class="live" role="group" aria-label={$t('searchSuggest')}>
          <span class="live-label">{$t('searchSuggest')}</span>
          <div class="sugg">
            {#each sugg.chips as word (word)}
              <button type="button" onpointerdown={(e) => e.preventDefault()} onclick={() => { interact('site_suggest', 'chip'); useExample(word); }}>{word}</button>
            {/each}
          </div>
        </div>
      {/if}

      {#if voiceNote}<p class="voice-note" role="status">{voiceNote}</p>{/if}
      <!-- Пока человек ищет слово, ответы прошлых вопросов не занимают лист: результаты важнее. -->
      {#if Talk && !(asked && shape === 'search')}<Talk compact={asked} onfollow={() => closePanel()} />{/if}

      {#if asked}
      <div class="scopes" role="group" aria-label={$t('searchLabel')}>
        {#each SCOPES as s (s.key)}
          {@const n = scopeCount(s.key)}
          <button
            type="button"
            class="sc"
            class:on={scope === s.key}
            aria-pressed={scope === s.key}
            disabled={asked && s.key !== 'all' && n === 0}
            onpointerdown={(e) => e.preventDefault()}
            onclick={() => { scope = s.key; input?.focus(); }}
          >
            <span class="ms">
              <svg class="wax" viewBox="0 0 52 52" aria-hidden="true"><path d={WAX_PATH} fill="#a2442a" /></svg>
              <SectionIcon name={s.icon} size={15} />
            </span>
            {$t(s.label as 'searchAll')}
            {#if n !== null}<i>{n}</i>{/if}
          </button>
        {/each}
      </div>
      {/if}

      <div class="body" id={listId} role="listbox" aria-label={$t('searchLabel')}>
        {#if matched.length}
          <section class="grp asks">
            <div class="gh">{$t('searchAsk')}</div>
            <div class="qs">
              {#each matched as q, i (q.key)}
                <button
                  type="button"
                  class="qr"
                  class:act={active === i}
                  role="option"
                  id="{listId}-{i}"
                  aria-selected={active === i}
                  data-row={i}
                  onpointerdown={(e) => e.preventDefault()}
                  onclick={() => choose(rows[i])}
                  onpointerenter={() => (active = i)}
                >{@render parts(q.question)}<span class="mk" aria-hidden="true">?</span></button>
              {/each}
            </div>
          </section>
        {/if}
        {#if !index}
          <p class="wait">{loadFailed ? $t('searchFailed') : $t('searchLooking')}</p>
        {:else if !asked}
          <div class="start" class:single={startMode !== 'home'}>
            {#if kit}
            <section>
              {#if startMode === 'home'}
                <h4>{$t('searchAsk')}</h4>
              {:else}
                <h4 class="back-row">
                  <button type="button" class="back" onpointerdown={(e) => e.preventDefault()} onclick={() => { startMode = 'home'; active = -1; input?.focus(); }}>← {$t('askBack')}</button>
                  <span>{startMode === 'all' ? $t('askAll') : $t(TOPIC_LABEL[startMode])}</span>
                </h4>
              {/if}
              {#if startMode === 'all'}
                {#each allGroups as g (g.topic)}
                  <h5 class="topic-head">{$t(TOPIC_LABEL[g.topic])}</h5>
                  <div class="qs">
                    {#each g.items as q, k (q.key)}
                      {@const i = g.at + k}
                      <button
                        type="button"
                        class="qr"
                        class:act={active === i}
                        role="option"
                        id="{listId}-{i}"
                        aria-selected={active === i}
                        data-row={i}
                        onpointerdown={(e) => e.preventDefault()}
                        onclick={() => choose(rows[i])}
                        onpointerenter={() => (active = i)}
                      >{q.question}<span class="mk" aria-hidden="true">?</span></button>
                    {/each}
                  </div>
                {/each}
              {:else}
                <div class="qs">
                  {#each questions as q, i (q)}
                    <button
                      type="button"
                      class="qr"
                      class:act={active === i}
                      role="option"
                      id="{listId}-{i}"
                      aria-selected={active === i}
                      data-row={i}
                      onpointerdown={(e) => e.preventDefault()}
                      onclick={() => choose(rows[i])}
                      onpointerenter={() => (active = i)}
                    >{q}<span class="mk" aria-hidden="true">?</span></button>
                  {/each}
                </div>
              {/if}
              {#if startMode === 'home' && startTopics.length}
                <div class="topics" role="group" aria-label={$t('askTopics')}>
                  {#each startTopics as topic (topic)}
                    <button type="button" onpointerdown={(e) => e.preventDefault()} onclick={() => openTopic(topic)}>{$t(TOPIC_LABEL[topic])}</button>
                  {/each}
                  <button type="button" class="all" onpointerdown={(e) => e.preventDefault()} onclick={() => openTopic('all')}>{$t('askAll')}</button>
                </div>
              {/if}
            </section>
            {/if}
            {#if freshTale && startMode === 'home'}
              {@const at = kit ? questions.length : 0}
              {@const copy = leafCopy(freshTale, $lang)}
              {@const mins = readingMinutes(copy.body)}
              <section>
                <h4>{$t('searchFreshTale')}</h4>
                <div class="rows">
                  <a href={leafHref(freshTale, SRC)} class="rw" class:act={active === at} role="option" id="{listId}-{at}" aria-selected={active === at} data-row={at} style:--i={at}
                    onclick={(e) => { e.preventDefault(); choose(rows[at]); }} onpointerenter={() => (active = at)}>
                    <span class="th">{#if leafCoverUrl(freshTale)}<img src={resolveMediaUrl(leafCoverUrl(freshTale))} alt="" loading="lazy" />{/if}</span>
                    <span class="tx"><span class="tt">{copy.title}</span>{#if copy.dek}<span class="dk">{copy.dek}</span>{/if}</span>
                    <span class="meta">{mins ? `${mins} ${$t(minutesKey(mins, $lang))}` : ''}</span>
                  </a>
                </div>
              </section>
            {/if}
          </div>
        {:else if found && foundTotal(found) === 0 && matched.length}
          <!-- Работ и небылиц нет, но подсказан вопрос: «не нашлось» рядом с ним спорило бы с ним. -->
        {:else if found && foundTotal(found) === 0}
          <div class="none">
            <h4>{$t('searchNone')}</h4>
            <p>{$t('searchNoneText').replace('{q}', query.trim())}</p>
            <div class="sugg">
              {#each $t('searchExampleWords').split('|').slice(0, 5) as word (word)}
                <button type="button" onpointerdown={(e) => e.preventDefault()} onclick={() => useExample(word)}>{word}</button>
              {/each}
            </div>
          </div>
        {:else if found}
          {#if worksShown.length}
            <section class="grp">
              <div class="gh">
                <SectionIcon name="works" size={16} />{$t('dockWorks')} <i>{found.works.length}</i>
                {#if found.works.length > worksShown.length}
                  <button type="button" class="more" onpointerdown={(e) => e.preventDefault()} onclick={() => (scope = 'works')}>{$t('searchMore').replace('{n}', String(found.works.length - worksShown.length))}</button>
                {/if}
              </div>
              <div class="arches">
                {#each worksShown as f, i (f.item.id)}{@render workPlate(f.item, f.why, workBase + i)}{/each}
              </div>
            </section>
          {/if}
          {#if talesShown.length}
            <section class="grp">
              <div class="gh"><SectionIcon name="tales" size={16} />{$t('navTales')} <i>{found.tales.length}</i></div>
              <div class="rows">
                {#each talesShown as f, i (f.item.id)}
                  {@const at = taleBase + i}
                  {@const copy = leafCopy(f.item, $lang)}
                  <a href={leafHref(f.item, SRC)} class="rw" class:act={active === at} role="option" id="{listId}-{at}" aria-selected={active === at} data-row={at} style:--i={at}
                    onclick={(e) => { e.preventDefault(); choose(rows[at]); }} onpointerenter={() => (active = at)}>
                    <span class="th">{#if leafCoverUrl(f.item)}<img src={resolveMediaUrl(leafCoverUrl(f.item))} alt="" loading="lazy" />{/if}</span>
                    <span class="tx"><span class="tt">{@render parts(copy.title)}</span>{#if copy.dek}<span class="dk">{@render parts(copy.dek)}</span>{/if}</span>
                    <span class="meta">{f.why.kind === 'name' ? '' : whyText(f.why)}</span>
                  </a>
                {/each}
              </div>
            </section>
          {/if}
          {#if gazetteShown.length}
            <section class="grp">
              <div class="gh"><SectionIcon name="gazette" size={16} />{$t('navGazette')} <i>{found.gazette.length}</i></div>
              <div class="rows">
                {#each gazetteShown as f, i (f.item.id)}
                  {@const at = leafBase + i}
                  {@const copy = leafCopy(f.item, $lang)}
                  <a href={leafHref(f.item, SRC)} class="rw" class:act={active === at} role="option" id="{listId}-{at}" aria-selected={active === at} data-row={at} style:--i={at}
                    onclick={(e) => { e.preventDefault(); choose(rows[at]); }} onpointerenter={() => (active = at)}>
                    <span class="th">{#if leafCoverUrl(f.item)}<img src={resolveMediaUrl(leafCoverUrl(f.item))} alt="" loading="lazy" />{/if}</span>
                    <span class="tx"><span class="tt">{@render parts(copy.title || figurineNameIn(f.item, $lang))}</span>{#if copy.dek}<span class="dk">{@render parts(copy.dek)}</span>{/if}</span>
                    <span class="meta">{quietDate(leafWhen(f.item), $lang)}</span>
                  </a>
                {/each}
              </div>
            </section>
          {/if}
          {#if cardsShown.length}
            <section class="grp">
              <div class="gh">
                <SectionIcon name="game" size={16} />{$t('searchCardsInGame')} <i>{found.cards.length}</i>
                {#if found.cards.length > cardsShown.length}
                  <button type="button" class="more" onpointerdown={(e) => e.preventDefault()} onclick={() => (scope = 'cards')}>{$t('searchMore').replace('{n}', String(found.cards.length - cardsShown.length))}</button>
                {/if}
              </div>
              <div class="cards">
                {#each cardsShown as f, i (f.item.id)}
                  {@const at = cardBase + i}
                  <a href={cardHref(f.item)} class="cd" class:act={active === at} role="option" id="{listId}-{at}" aria-selected={active === at} data-row={at} style:--i={at}
                    onclick={(e) => { e.preventDefault(); choose(rows[at]); }} onpointerenter={() => (active = at)}>
                    <span class="cf">
                      {#if f.item.artUrl}<img src={resolveMediaUrl(f.item.artUrl)} alt="" loading="lazy" />{/if}
                      <span class="tier" aria-hidden="true">{#each { length: f.item.tier } as _, k (k)}<b></b>{/each}</span>
                    </span>
                    <span class="tt">{@render parts(cardTitle(f.item, $lang))}</span>
                    <span class="why">{f.why.kind === 'race' ? f.why.race : whyText(f.why)}</span>
                  </a>
                {/each}
              </div>
            </section>
          {/if}
        {/if}
      </div>

      <div class="foot">
        <a href={archiveHref} onclick={() => { kit?.siteAsk.dropLive(); closePanel(); }}>{asked ? $t('searchAllMatches').replace('{q}', query.trim()) : $t('searchWholeArchive')}</a>
        <span class="keys" aria-hidden="true">
          <span><kbd>↑↓</kbd>{$t('searchKeyPick')}</span>
          <span><kbd>↵</kbd>{$t('searchKeyOpen')}</span>
          <span><kbd>esc</kbd>{$t('searchKeyClose')}</span>
        </span>
      </div>
    </div>
  {/if}
</div>

<style>
  .ss,
  .panel {
    --ss-paper: #fbf7ef;
    --ss-line: var(--color-border-soft, #d8c6b1);
    --ss-rule: #b9a184;
    --ss-ink: var(--color-ink-primary, #34251c);
    --ss-ink-2: var(--color-ink-secondary, #5a3420);
    --ss-ink-3: var(--color-ink-tertiary, #7a5035);
    --ss-ember: var(--color-ember, #b44e24);
    --ss-ember-deep: var(--color-ember-deep, #9a4120);
    font-family: var(--font-body, Inter, system-ui, sans-serif);
  }
  .ss {
    position: relative;
  }

  /* ── Поле ── */
  .ss--field {
    width: 100%;
    max-width: 620px;
  }
  .field {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 38px;
  }
  .loupe {
    display: grid;
    color: var(--ss-ink-3);
    --ib: var(--color-canvas-base, #f8f1e7);
    transition: color 0.3s ease, transform 0.5s cubic-bezier(0.3, 1.4, 0.5, 1);
  }
  .ss.on .loupe {
    color: var(--ss-ember-deep);
    transform: rotate(-14deg);
  }
  .input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    padding: 0;
    background: none;
    outline: none;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 20px;
    color: var(--ss-ink);
    caret-color: var(--ss-ember);
    -webkit-appearance: none;
    appearance: none;
  }
  /* Общий стиль полей сайта рисует вокруг фокуса кольцо; здесь фокус
     показывает черта под полем, наливающаяся красным. */
  .input:focus,
  .input:focus-visible {
    outline: none;
    box-shadow: none;
    border-radius: 0;
  }
  .input::-webkit-search-cancel-button {
    display: none;
  }
  .hint {
    position: absolute;
    left: 34px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 19px;
    color: var(--ss-ink-3);
    white-space: nowrap;
    overflow: hidden;
    max-width: calc(100% - 70px);
  }
  .field--panel .hint {
    left: 50px;
  }
  /* Хвост подсказки: набранное невидимо и держит место, дальше серый остаток. */
  .ghost {
    position: absolute;
    left: 34px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 20px;
    color: color-mix(in srgb, var(--ss-ink-3) 70%, transparent);
    white-space: pre;
    overflow: hidden;
    max-width: calc(100% - 100px);
  }
  .field--panel .ghost {
    left: 50px;
  }
  .ghost-typed {
    color: transparent;
  }
  .ghost-key {
    margin-left: 10px;
    padding: 0 5px;
    border: 1px solid var(--ss-line);
    border-radius: 3px;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-style: normal;
    font-size: 10px;
    letter-spacing: 0.04em;
    color: var(--ss-ink-3);
    vertical-align: 3px;
  }
  .live {
    display: flex;
    align-items: baseline;
    gap: 14px;
    padding: 12px 26px 2px;
  }
  .live-label {
    flex: none;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ss-ink-3);
  }
  .hint-word {
    display: inline-block;
    font-style: italic;
    /* ink-3 на бумаге 6.2:1; прежние #8f6f52 давали 4.1:1. */
    color: var(--ss-ink-3);
  }
  .key {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 13px;
    color: var(--ss-ink-3);
    box-shadow:
      inset 1px 1.5px 1.5px rgba(111, 59, 36, 0.2),
      inset -1px -1px 1px rgba(255, 255, 255, 0.95),
      0 0 0 1px rgba(156, 125, 93, 0.3);
  }
  .ss.on .key {
    opacity: 0;
  }
  .clear {
    flex: none;
    width: 26px;
    height: 26px;
    border: 0;
    background: none;
    font-size: 20px;
    line-height: 1;
    color: var(--ss-ink-3);
    cursor: pointer;
  }
  .clear:hover {
    color: var(--ss-ember-deep);
  }
  .ink {
    position: relative;
    display: block;
    height: 1.5px;
    margin-left: 34px;
    /* Черта под полем — единственная граница поля: прежние #b9a184 давали 2.3:1,
       и поле читалось подсказкой, а не местом ввода. Теперь 3.5:1. */
    background: #8f7355;
  }
  .ink::after {
    content: '';
    position: absolute;
    inset: -0.5px 0;
    background: var(--ss-ember);
    transform: scaleX(0);
    transform-origin: left;
    transition: transform 0.6s cubic-bezier(0.2, 0.7, 0.2, 1);
  }
  .ss.on .ink::after {
    transform: scaleX(1);
  }
  /* ── Лупа ── */
  .trigger {
    position: relative;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--ss-ink-2);
    cursor: pointer;
    --ib: var(--color-canvas-base, #f8f1e7);
    transition: color 0.2s ease, background 0.2s ease;
  }
  @media (max-width: 1024px), (pointer: coarse) {
    .trigger::after {
      content: '';
      position: absolute;
      left: 50%;
      top: 50%;
      width: 44px;
      height: 44px;
      transform: translate(-50%, -50%);
    }
  }
  .trigger:hover,
  .ss.on .trigger {
    color: var(--ss-ember-deep);
    background: color-mix(in srgb, var(--ss-ember) 8%, transparent);
  }

  /* ── Лист выдачи ── */
  .panel {
    position: absolute;
    z-index: 260;
    left: -24px;
    top: calc(100% + 14px);
    width: min(820px, calc(100vw - 40px));
    max-height: min(640px, calc(100vh - 150px));
    display: flex;
    flex-direction: column;
    background: var(--ss-paper);
    box-shadow:
      0 0 0 1px var(--ss-rule),
      0 0 0 5px var(--ss-paper),
      0 0 0 6px var(--ss-line),
      0 26px 50px -22px rgba(60, 30, 15, 0.45);
    animation: unfold 0.38s cubic-bezier(0.2, 0.7, 0.2, 1) both;
    color: var(--ss-ink);
  }
  @keyframes unfold {
    from { clip-path: inset(0 0 100% 0); opacity: 0.4; }
    to { clip-path: inset(-12px -12px -60px -12px); opacity: 1; }
  }
  .defs {
    position: absolute;
    width: 0;
    height: 0;
  }
  .panel--quiet {
    animation: none;
  }
  /* Лупа на компьютере (шапка свёрнута): лист под шапкой, по середине окна. */
  .panel--float {
    position: fixed;
    left: 50%;
    top: calc(var(--site-header-h, 56px) + 12px);
    transform: translateX(-50%);
  }
  .field--panel {
    flex: none;
    height: 56px;
    padding: 0 22px;
    border-bottom: 1px solid var(--ss-rule);
  }
  .close {
    flex: none;
    border: 0;
    background: none;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 15px;
    color: var(--ss-ember-deep);
    cursor: pointer;
  }

  .scopes {
    flex: none;
    display: flex;
    gap: 4px;
    align-items: center;
    margin: 0 26px;
    padding: 16px 0 14px;
    border-bottom: 1px solid var(--ss-line);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .sc {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 4px 4px;
    border: 0;
    border-radius: 20px;
    background: none;
    cursor: pointer;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 15px;
    color: var(--ss-ink-2);
    transition: background 0.2s ease;
  }
  .sc:hover:not(:disabled) {
    background: #f1e6d6;
  }
  .sc:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .ms {
    position: relative;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: #8f6f52;
    --ib: var(--ss-paper);
    --em: #8f6f52;
    box-shadow:
      inset 1px 1.5px 1.5px rgba(111, 59, 36, 0.22),
      inset -1px -1px 1px rgba(255, 255, 255, 0.95),
      0 0 0 1px rgba(156, 125, 93, 0.3);
  }
  .ms .wax {
    position: absolute;
    inset: -3px;
    width: 34px;
    height: 34px;
    opacity: 0;
    transition: opacity 0.25s ease;
  }
  .ms :global(.section-icon) {
    position: relative;
  }
  .sc.on .ms {
    box-shadow: none;
    color: #6a2210;
    --ib: #a2442a;
    --em: #e9a07a;
  }
  .sc.on .ms .wax {
    opacity: 1;
    filter: drop-shadow(0 1.5px 1.5px rgba(80, 30, 12, 0.3));
  }
  .sc.on {
    color: var(--ss-ember-deep);
    font-style: italic;
  }
  .sc i {
    font-size: 13px;
    color: var(--ss-ink-3);
    font-variant-numeric: oldstyle-nums;
  }

  .body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding-bottom: 12px;
  }
  .wait {
    padding: 28px 26px;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    color: var(--ss-ink-3);
  }
  h4,
  .gh {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.24em;
    text-transform: uppercase;
    color: var(--ss-ink-3);
  }
  h4 {
    margin: 0 0 12px;
  }
  .gh {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
    --ib: var(--ss-paper);
    --em: var(--ss-ink-3);
  }
  .gh i {
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    font-size: 13px;
  }
  .gh .more {
    margin-left: auto;
    border: 0;
    background: none;
    cursor: pointer;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    font-size: 14px;
    color: var(--ss-ember-deep);
  }
  .grp {
    padding: 18px 26px 4px;
  }
  .start {
    display: grid;
    grid-template-columns: 1.25fr 1fr;
    gap: 34px;
    padding: 26px 26px 10px;
  }
  .start.single {
    grid-template-columns: 1fr;
  }
  .qs {
    display: flex;
    flex-direction: column;
  }
  .back-row {
    display: flex;
    align-items: baseline;
    gap: 14px;
  }
  .back {
    border: 0;
    background: none;
    padding: 2px 0;
    cursor: pointer;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    color: var(--ss-ember-deep);
  }
  .back:hover,
  .back:focus-visible {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .topic-head {
    margin: 16px 0 2px;
    padding-left: 12px;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ss-ink-3);
  }
  .topics {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
    padding: 0 12px;
  }
  .topics button {
    padding: 5px 12px;
    border: 1px solid var(--ss-line);
    border-radius: 14px;
    background: none;
    cursor: pointer;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 12.5px;
    color: var(--ss-ink-2);
    transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
  }
  .topics button:hover,
  .topics button:focus-visible {
    color: var(--ss-ember-deep);
    border-color: var(--ss-ember);
    background: color-mix(in srgb, var(--ss-ember) 7%, transparent);
  }
  .topics button:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 2px;
  }
  .topics .all {
    border-style: dashed;
  }
  .asks {
    padding-bottom: 6px;
  }
  .qr {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 12px;
    border: 0;
    border-left: 2px solid transparent;
    background: none;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 17px;
    color: var(--ss-ink);
  }
  .qr .mk {
    margin-left: auto;
    font-style: normal;
    color: var(--ss-ember-deep);
  }
  .qr.act,
  .qr:hover {
    background: color-mix(in srgb, var(--ss-ember) 7%, transparent);
    border-left-color: var(--ss-ember);
  }
  .qr:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: -2px;
  }
  .none {
    padding: 28px 26px 10px;
  }
  .mic {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid transparent;
    border-radius: 50%;
    background: none;
    color: var(--ss-ink-3);
    cursor: pointer;
    transition: color 0.2s ease, background 0.2s ease, border-color 0.2s ease;
  }
  .mic:hover {
    color: var(--ss-ember-deep);
    border-color: var(--ss-line);
  }
  .mic.on {
    color: #fbf7ef;
    background: var(--ss-ember-deep);
    animation: listen 1.4s ease-in-out infinite;
  }
  @keyframes listen {
    50% { box-shadow: 0 0 0 5px color-mix(in srgb, var(--ss-ember) 25%, transparent); }
  }
  .mic:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 2px;
  }
  .voice-note {
    margin: 0;
    padding: 8px 26px 0;
    font-size: 12.5px;
    color: var(--ss-ember-deep);
  }
  .none p {
    margin: 0 0 14px;
    font-family: Georgia, serif;
    font-size: 16px;
    color: var(--ss-ink-2);
  }
  .sugg {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
  }
  .sugg button {
    border: 0;
    padding: 2px 0;
    background: none;
    cursor: pointer;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 18px;
    color: var(--ss-ink);
    box-shadow: inset 0 -1px 0 #c9b396;
    transition: box-shadow 0.2s ease, color 0.2s ease;
  }
  .sugg button:hover {
    color: var(--ss-ember-deep);
    box-shadow: inset 0 -1px 0 var(--ss-ember);
  }

  /* Работы — фотографии в стрельчатой арке, как на полке небылиц. */
  .arches {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 14px;
  }
  .arches--four {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .wk,
  .rw,
  .cd {
    color: var(--ss-ink);
    text-decoration: none;
    animation: rise 0.4s cubic-bezier(0.2, 0.7, 0.2, 1) both;
    animation-delay: calc(min(var(--i), 12) * 28ms);
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: none; }
  }
  .wk {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .ph {
    position: relative;
    width: 100%;
    aspect-ratio: 4 / 5;
  }
  .ph img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    clip-path: var(--arch);
    background: #e9dcc8;
  }
  .arch-line {
    position: absolute;
    inset: -4px;
    width: calc(100% + 8px);
    height: calc(100% + 8px);
    overflow: visible;
    pointer-events: none;
  }
  .arch-line path {
    fill: none;
    stroke: var(--ss-rule);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
    transition: stroke 0.2s ease;
  }
  .sold {
    position: absolute;
    right: -6px;
    bottom: 6px;
    width: 22px;
    height: 22px;
  }
  .nm {
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 13.5px;
    line-height: 1.2;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .why {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ss-ink-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wk.act .arch-line path,
  .wk:hover .arch-line path {
    stroke: var(--color-ember-ink, #6f3b24);
    stroke-width: 1.6;
  }
  .wk.act .nm {
    color: var(--ss-ember-deep);
  }

  .rows {
    display: flex;
    flex-direction: column;
  }
  .rw {
    position: relative;
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr) auto;
    gap: 14px;
    align-items: center;
    padding: 8px 10px 8px 12px;
    margin: 0 -12px;
  }
  .rw::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    width: 6px;
    height: 6px;
    margin-top: -3px;
    transform: rotate(45deg) scale(0);
    background: var(--ss-ember);
    transition: transform 0.2s ease;
  }
  .rw.act,
  .rw:hover {
    background: #f3e9da;
  }
  .rw.act::before {
    transform: rotate(45deg) scale(1);
  }
  .th {
    width: 44px;
    height: 54px;
  }
  .th img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    clip-path: var(--arch);
  }
  .tx {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tt {
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 17px;
    line-height: 1.2;
  }
  .rw .tt {
    font-style: italic;
  }
  .dk {
    font-family: Georgia, serif;
    font-size: 13.5px;
    color: var(--ss-ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ss-ink-3);
    white-space: nowrap;
  }

  /* Карты — в рамке карты, ранг ромбами. */
  .cards {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 14px;
  }
  .cd {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .cf {
    position: relative;
    aspect-ratio: 5 / 7;
    padding: 5px;
    border-radius: 3px;
    background: #f1e6d6;
    box-shadow:
      inset 0 0 0 1px #9c7d5d,
      inset 0 0 0 3px #f1e6d6,
      inset 0 0 0 4px #c9b396;
  }
  .cf img {
    display: block;
    width: 100%;
    height: 72%;
    object-fit: cover;
    border-radius: 1px;
  }
  .tier {
    position: absolute;
    left: 50%;
    bottom: 8px;
    transform: translateX(-50%);
    display: flex;
    gap: 4px;
  }
  .tier b {
    width: 5px;
    height: 5px;
    transform: rotate(45deg);
    background: #9c7d5d;
  }
  .cd .tt {
    font-size: 13.5px;
  }
  .cd.act .cf,
  .cd:hover .cf {
    box-shadow:
      inset 0 0 0 1.6px var(--color-ember-ink, #6f3b24),
      inset 0 0 0 3px #f1e6d6,
      inset 0 0 0 4px #c9b396;
  }
  .cd.act .tt {
    color: var(--ss-ember-deep);
  }

  /* Совпадение — красная черта под словом, без заливки. */
  mark {
    background: none;
    color: inherit;
    box-shadow: inset 0 -1px 0 var(--ss-ember);
  }
  .tt mark,
  .nm mark {
    color: var(--ss-ember-deep);
  }

  .foot {
    flex: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    padding: 13px 26px;
    border-top: 1px solid var(--ss-line);
  }
  .foot a {
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 16px;
    color: var(--ss-ember-deep);
    text-decoration: none;
  }
  .foot a:hover {
    text-decoration: underline;
    text-underline-offset: 4px;
    text-decoration-thickness: 1px;
  }
  .keys {
    display: flex;
    gap: 16px;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ss-ink-3);
  }
  .keys kbd {
    margin-right: 5px;
    padding: 0 5px;
    border-radius: 3px;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 12px;
    letter-spacing: 0;
    color: var(--ss-ink-2);
    box-shadow: 0 0 0 1px #c9b396;
  }

  .wk:focus-visible,
  .rw:focus-visible,
  .cd:focus-visible,
  .sc:focus-visible,
  .trigger:focus-visible,
  .sugg button:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 3px;
  }

  /* ── Телефон: лист во весь экран ── */
  @media (max-width: 1024px) {
    .panel--float {
      position: fixed;
      inset: 0;
      top: 0;
      left: 0;
      transform: none;
      width: 100%;
      max-height: none;
      z-index: 320;
      box-shadow: none;
      padding-top: env(safe-area-inset-top, 0px);
    }
    .field--panel {
      height: 60px;
      padding: 0 16px;
    }
    .scopes {
      margin: 0;
      padding: 10px 12px;
      gap: 2px;
    }
    .sc {
      font-size: 13.5px;
      padding-right: 8px;
    }
    .grp {
      padding: 14px 16px 2px;
    }
    .start {
      grid-template-columns: 1fr;
      gap: 24px;
      padding: 18px 16px;
    }
    .none {
      padding: 22px 16px;
    }
    .arches {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 12px;
    }
    .cards {
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 10px;
    }
    .rw {
      grid-template-columns: 40px minmax(0, 1fr);
    }
    .rw .meta,
    .keys {
      display: none;
    }
    .foot {
      padding: 12px 16px calc(12px + env(safe-area-inset-bottom, 0px));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .panel,
    .wk,
    .rw,
    .cd {
      animation: none;
    }
    .loupe,
    .ink::after {
      transition: none;
    }
  }
</style>
