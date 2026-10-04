<script lang="ts">
  // The writing desk.
  //
  // Not a form. The gazette composer next door is a newsroom — kinds, feeds,
  // cuttings, templates — and prose written inside it comes out sounding like
  // a form filled in. Here the desk IS the paper: the title is typed in the
  // face it will be read in, the body in the measure it will be read at, and
  // the work stands in the margin exactly where the visitor will meet it.
  import { onMount, onDestroy, tick } from 'svelte';
  import { api, ApiError } from '$lib/api';
  import { t, lang, type Lang } from '$lib/i18n';
  import { TITLE_MAX, DEK_MAX, BODY_MAX, AUTHOR_MAX, workFrameUrls, leafHref } from '$lib/gazette';
  import { ORNAMENT, readingMinutes, minutesKey, renderTale } from '$lib/tales';
  import { resolveSiteRefs, siteRefsIn, type SiteRefInfo } from '$lib/siteLinks';
  import TaleProse from '../TaleProse.svelte';
  import TalePollDesk from './TalePollDesk.svelte';
  import type {
    AdminSequelWish,
    AdminTaleCommentDto,
    AdminTaleStat,
    TaleNoticeStatus,
    Figurine,
    FigurineListItem,
    GazetteLeaf,
    GazetteSeed,
    GazetteStatus,
    SaveGazetteLeafRequest,
    TaleLettersForecast,
  } from '$lib/types/api';

  let {
    seed = null,
    onSeedConsumed,
  }: { seed?: GazetteSeed | null; onSeedConsumed?: () => void } = $props();

  /** A little story stops being little somewhere around here. */
  const LITTLE = 2500;
  const AUTOSAVE_MS = 2500;
  const REORDER_MS = 600;
  const DRAFT_PREFIX = 'gotiga_tale_draft_';

  let tales = $state<GazetteLeaf[]>([]);
  let figurines = $state<FigurineListItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let savedAt = $state<Date | null>(null);
  let message = $state('');
  let listQuery = $state('');
  let ready = $state(false);

  let selectedId = $state<string | null>(null);
  let titleEn = $state('');
  let titleRu = $state('');
  let dekEn = $state('');
  let dekRu = $state('');
  let bodyEn = $state('');
  let bodyRu = $state('');
  let figurineId = $state('');
  // Кем написана история. Одно поле на оба языка: имя не переводится.
  let author = $state('');
  let imageUrls = $state<string[]>([]);
  let slug = $state('');
  let status = $state<GazetteStatus>('draft');
  let pinned = $state(false);
  let scheduledAt = $state<string | null>(null);
  let updatedAt = $state('');

  // English is the source of truth for this house, so the desk opens in it.
  let editLang = $state<Lang>('en');
  let both = $state(false);
  let focus = $state(false);
  let sealed = $state(false);
  let uploading = $state(false);

  let figQuery = $state('');
  let figOpen = $state(false);
  let restorable = $state<Record<string, unknown> | null>(null);
  // Full figurine detail, fetched for its photo gallery — the list row only
  // carries the face photo, and the picker below needs the whole set.
  let extraFig = $state<Figurine | null>(null);

  /**
   * Отклик читателя. Свод приходит одним запросом на все байки: строка списка
   * показывает, есть ли непрочитанный отклик, и ради каждой строки отдельного
   * запроса не делается.
   */
  let stats = $state<Record<string, AdminTaleStat>>({});
  let comments = $state<AdminTaleCommentDto[]>([]);
  let commentsLoading = $state(false);
  let talkOpen = $state(false);
  let replyFor = $state<string | null>(null);
  let replyText = $state('');

  /** Стол голосования «о ком следующая» стоит на месте бумаги, пока открыт. */
  let pollDesk = $state(false);

  /**
   * Кто ждёт продолжения открытой байки и дошло ли до него известие. Грузится
   * по нажатию: список нужен, когда продолжение пишут или только что выложили,
   * а не на каждом открытии байки.
   */
  let waitingOpen = $state(false);
  let waiting = $state<AdminSequelWish[]>([]);
  let waitingLoading = $state(false);

  /**
   * Листок перед выходом: что с байкой не так и что уйдёт людям. Выход —
   * единственное действие стола, которое нельзя забрать назад: через десять
   * минут байка уходит письмом всей книге дома. Поэтому число писем и
   * замечания показываются до нажатия, а не находятся потом на сайте.
   */
  let preflight = $state<{
    target: 'published' | 'scheduled';
    at: string | null;
    warnings: string[];
    forecast: TaleLettersForecast | null;
  } | null>(null);
  let preflightBusy = $state(false);
  /** «Запланировано» нажато, дата ещё не выбрана — статус пока прежний. */
  let scheduling = $state(false);
  /** Подсказка о том, что понимает тело байки. */
  let markupOpen = $state(false);

  /**
   * Бумага «как у читателя»: тот же TaleProse, что на странице байки, вместо
   * полей ввода. Названия ссылок спрашиваются так же, как их спрашивает
   * загрузчик страницы, — иначе предпросмотр печатал бы адрес там, где
   * читатель увидит название.
   */
  let reading = $state(false);
  let readingLinks = $state<Record<string, SiteRefInfo>>({});

  /**
   * Адрес байки закреплён, когда о ней ушли письма: он стоит в этих письмах.
   * Сервер держит то же правило сам, здесь — чтобы не предлагать правку,
   * которую он не примет.
   */
  let addressFixed = $state(false);
  let slugDraft = $state('');

  let bodyBox = $state<HTMLTextAreaElement | null>(null);
  let secondBox = $state<HTMLTextAreaElement | null>(null);
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let reorderTimer: ReturnType<typeof setTimeout> | null = null;
  let sealTimer: ReturnType<typeof setTimeout> | null = null;
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let syncing = false;

  let selectedFig = $derived(figurines.find((f) => f.id === figurineId) ?? null);
  let plate = $derived(imageUrls[0] ?? '');
  let frames = $derived(selectedFig ? workFrameUrls(selectedFig, extraFig) : []);
  let titleNow = $derived(editLang === 'ru' ? titleRu : titleEn);
  let dekNow = $derived(editLang === 'ru' ? dekRu : dekEn);
  let bodyNow = $derived(editLang === 'ru' ? bodyRu : bodyEn);
  let otherLang = $derived<Lang>(editLang === 'ru' ? 'en' : 'ru');
  let otherTitle = $derived(editLang === 'ru' ? titleEn : titleRu);
  let otherBody = $derived(editLang === 'ru' ? bodyEn : bodyRu);
  let chars = $derived(bodyNow.length);
  let fill = $derived(Math.min(1, chars / LITTLE));
  let tooLong = $derived(chars > LITTLE);
  let open = $derived(selectedId !== null || titleEn !== '' || titleRu !== '');
  let mine = $derived(selectedId ? stats[selectedId] ?? null : null);
  let minutes = $derived(readingMinutes(bodyNow));
  /** Какая половина сайта осталась без текста. */
  let bodyMissing = $derived<Record<Lang, boolean>>({ en: !bodyEn.trim(), ru: !bodyRu.trim() });
  /** Адрес на сайте. Ссылка открывается только у байки, которая на людях. */
  let siteHref = $derived(selectedId && slug ? leafHref({ kind: 'tale', slug } as GazetteLeaf) : '');
  /** На людях ли байка сейчас: опубликована или её час уже наступил. */
  let onDisplay = $derived(
    status === 'published' || (status === 'scheduled' && !!scheduledAt && new Date(scheduledAt) <= new Date()),
  );
  let slugEditable = $derived(!!selectedId && !onDisplay && !addressFixed);

  let visible = $derived.by(() => {
    const q = listQuery.trim().toLowerCase();
    if (!q) return tales;
    return tales.filter((tale) =>
      `${tale.titleEn} ${tale.titleRu} ${tale.figurineName ?? ''}`.toLowerCase().includes(q),
    );
  });
  let figMatches = $derived.by(() => {
    const q = figQuery.trim().toLowerCase();
    const list = q ? figurines.filter((f) => f.name.toLowerCase().includes(q)) : figurines;
    return list.slice(0, 12);
  });

  function fieldsKey(): string {
    return JSON.stringify({
      titleEn, titleRu, dekEn, dekRu, bodyEn, bodyRu, author,
      figurineId, imageUrls, slug, status, pinned, scheduledAt,
    });
  }
  let key = $derived(fieldsKey());
  let snapshot = $state('');
  let dirty = $derived(ready && key !== snapshot);

  function flash(text: string, ms = 4000) {
    message = text;
    setTimeout(() => {
      if (message === text) message = '';
    }, ms);
  }

  function draftKey(): string {
    return `${DRAFT_PREFIX}${selectedId ?? 'new'}`;
  }

  // ── Loading ────────────────────────────────────────────────────────────────

  /**
   * The shelf, in shelf order — the same order the room shows.
   *
   * The admin listing comes back newest-first, which is right for a newsroom
   * and wrong here: dragging in a recency-sorted list would write shelf
   * positions taken from an order nobody arranged, and every save would
   * quietly reshuffle the shelf. A tale with no place yet waits at the end.
   */
  async function loadTales() {
    const page = await api.adminListGazetteLeaves({ kind: 'tale', perPage: 200 });
    tales = [...page.items].sort((a, b) => {
      const ao = a.shelfOrder ?? Number.MAX_SAFE_INTEGER;
      const bo = b.shelfOrder ?? Number.MAX_SAFE_INTEGER;
      if (ao !== bo) return ao - bo;
      return (b.publishedAt ?? b.createdAt).localeCompare(a.publishedAt ?? a.createdAt);
    });
  }

  /** Свод отклика по всем байкам разом. Тихий: числа — не содержимое стола. */
  async function loadStats() {
    try {
      const rows = await api.adminTaleStats();
      stats = Object.fromEntries(rows.map((row) => [row.taleId, row]));
    } catch {
      stats = {};
    }
  }

  async function loadComments(taleId: string) {
    commentsLoading = true;
    try {
      const page = await api.adminListTaleComments({ taleId, perPage: 100, sort: 'newest' });
      comments = page.items;
    } catch (e) {
      flash(String(e), 6000);
      comments = [];
    } finally {
      commentsLoading = false;
    }
  }

  async function toggleWaiting() {
    waitingOpen = !waitingOpen;
    if (!waitingOpen || !selectedId) return;
    waitingLoading = true;
    try {
      waiting = await api.adminTaleWaiting(selectedId);
    } catch (e) {
      flash(String(e), 6000);
      waiting = [];
    } finally {
      waitingLoading = false;
    }
  }

  const NOTICE_WORD: Record<TaleNoticeStatus, string> = {
    none: 'adminTalesWaitingNone',
    pending: 'adminTalesWaitingPending',
    sent: 'adminTalesWaitingSent',
    failed: 'adminTalesWaitingFailed',
  };

  /** Одобрить, спрятать или ответить. Ответ уходит вместе с одобрением. */
  async function moderate(c: AdminTaleCommentDto, isApproved: boolean, adminReply?: string | null) {
    try {
      await api.adminModerateTaleComment(c.id, {
        isApproved,
        adminReply: adminReply === undefined ? c.adminReply : adminReply,
      });
      if (selectedId) {
        await loadComments(selectedId);
        await loadStats();
      }
      replyFor = null;
      replyText = '';
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  async function removeComment(c: AdminTaleCommentDto) {
    if (!confirm($t('adminTalesCommentDeleteAsk'))) return;
    try {
      await api.adminDeleteTaleComment(c.id);
      if (selectedId) {
        await loadComments(selectedId);
        await loadStats();
      }
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  function blank() {
    selectedId = null;
    titleEn = '';
    titleRu = '';
    dekEn = '';
    dekRu = '';
    bodyEn = '';
    bodyRu = '';
    figurineId = '';
    author = '';
    imageUrls = [];
    slug = '';
    status = 'draft';
    pinned = false;
    scheduledAt = null;
    updatedAt = '';
    savedAt = null;
    figQuery = '';
    restorable = null;
    extraFig = null;
    comments = [];
    talkOpen = false;
    replyFor = null;
    waitingOpen = false;
    waiting = [];
    preflight = null;
    scheduling = false;
    addressFixed = false;
    slugDraft = '';
    snapshot = fieldsKey();
  }

  function apply(leaf: GazetteLeaf) {
    selectedId = leaf.id;
    titleEn = leaf.titleEn;
    titleRu = leaf.titleRu;
    dekEn = leaf.dekEn ?? '';
    dekRu = leaf.dekRu ?? '';
    bodyEn = leaf.bodyEn ?? '';
    bodyRu = leaf.bodyRu ?? '';
    figurineId = leaf.figurineId ?? '';
    author = leaf.author ?? '';
    imageUrls = leaf.imageUrls?.length ? leaf.imageUrls.filter(Boolean) : leaf.imageUrl ? [leaf.imageUrl] : [];
    slug = leaf.slug;
    status = leaf.status;
    pinned = leaf.pinned;
    scheduledAt = leaf.scheduledAt;
    updatedAt = leaf.updatedAt;
    figQuery = '';
    snapshot = fieldsKey();
  }

  function openTale(leaf: GazetteLeaf) {
    if (dirty && !confirm($t('adminTalesUnsavedLeave'))) return;
    pollDesk = false;
    preflight = null;
    scheduling = false;
    apply(leaf);
    savedAt = null;
    comments = [];
    replyFor = null;
    waitingOpen = false;
    waiting = [];
    void loadComments(leaf.id);
    void loadAddressFixed(leaf.id);
    // A crash, a closed tab, a browser that went away mid-sentence: whatever is
    // in this browser wins only if it is newer than what the server holds.
    restorable = null;
    try {
      const raw = localStorage.getItem(draftKey());
      if (raw) {
        const draft = JSON.parse(raw) as { at?: string };
        if (draft?.at && new Date(draft.at) > new Date(leaf.updatedAt)) restorable = draft;
      }
    } catch {
      restorable = null;
    }
  }

  function restoreDraft() {
    const d = restorable as Record<string, string | string[] | boolean | null> | null;
    if (!d) return;
    titleEn = (d.titleEn as string) ?? titleEn;
    titleRu = (d.titleRu as string) ?? titleRu;
    dekEn = (d.dekEn as string) ?? dekEn;
    dekRu = (d.dekRu as string) ?? dekRu;
    bodyEn = (d.bodyEn as string) ?? bodyEn;
    bodyRu = (d.bodyRu as string) ?? bodyRu;
    author = (d.author as string) ?? author;
    // Работа и фотография — тоже часть написанного: восстановленный текст
    // без них вернул бы байку к чужой обложке.
    figurineId = (d.figurineId as string) ?? figurineId;
    if (Array.isArray(d.imageUrls)) imageUrls = d.imageUrls as string[];
    restorable = null;
  }

  function dropDraft() {
    localStorage.removeItem(draftKey());
    restorable = null;
  }

  function startNew() {
    if (dirty && !confirm($t('adminTalesUnsavedLeave'))) return;
    blank();
    pollDesk = false;
  }

  /** Читатели выбрали работу — стол открывается с ней, как из формы работы. */
  function writeChosen(id: string) {
    if (dirty && !confirm($t('adminTalesUnsavedLeave'))) return;
    blank();
    figurineId = id;
    const face = figurines.find((f) => f.id === id)?.faceImageUrl;
    if (face) imageUrls = [face];
    pollDesk = false;
  }

  /**
   * Какую байку продолжает эта. Ручка своя, а не поле формы, как у порядка
   * полки: это отношение между байками, и автосохранение листа его не трогает.
   * Если продолжение уже вышло, просившие получат письмо сразу — это решает
   * сервер.
   */
  async function setSequel(select: HTMLSelectElement) {
    if (!selectedId) return;
    const of = select.value || null;
    try {
      await api.adminSetTaleSequel(selectedId, of);
      await loadStats();
      flash(of ? $t('adminTalesSequelSaved') : $t('adminTalesSequelCleared'));
    } catch (e) {
      // Не сохранилось — список возвращается к тому, что лежит на сервере:
      // `value` у него не менялся, и сам Svelte его не вернёт, а список,
      // показывающий несохранённый выбор, хуже неизменившегося.
      select.value = mine?.sequelOf ?? '';
      flash(
        e instanceof ApiError && e.status === 400 ? $t('adminTalesSequelRefused') : String(e),
        6000,
      );
    }
  }

  async function loadAddressFixed(id: string) {
    addressFixed = false;
    try {
      const f = await api.adminTaleLettersForecast(id);
      if (selectedId === id) addressFixed = f.laid;
    } catch {
      // Не узнали — правку разрешает сервер или молча отклоняет.
    }
  }

  /**
   * Адрес правится отдельным полем и уходит по выходу из него, а не с
   * автосохранением: сервер приводит адрес к виду слага (срезает дефис на
   * конце, заменяет пробелы), и ответ, пришедший посреди набора, переписывал
   * бы поле под пальцами.
   */
  async function commitSlug() {
    const next = slugDraft.trim();
    if (!next || next === slug) {
      slugDraft = slug;
      return;
    }
    slug = next;
    if (autosaveTimer) clearTimeout(autosaveTimer);
    if (await save()) {
      slugDraft = slug;
      if (slug !== next) flash($t('adminTalesSlugAdjusted').replace('{slug}', slug));
    }
  }

  $effect(() => {
    // Поле показывает сохранённый адрес, пока его не трогают.
    if (document.activeElement?.classList.contains('slug-input')) return;
    slugDraft = slug;
  });

  $effect(() => {
    if (!reading) return;
    const refs = siteRefsIn(`${bodyEn}\n${bodyRu}`);
    if (!refs.length) {
      readingLinks = {};
      return;
    }
    let live = true;
    void resolveSiteRefs(refs, {
      getFigurine: (handle) => api.getFigurine(handle),
      getGazetteLeaf: (s) => api.getGazetteLeaf(s),
    }).then((found) => {
      if (live) readingLinks = found;
    });
    return () => {
      live = false;
    };
  });

  // ── Saving ─────────────────────────────────────────────────────────────────

  function copyOr(a: string, b: string): string {
    return a.trim() || b.trim();
  }

  function payload(): SaveGazetteLeafRequest {
    return {
      slug: slug.trim() || null,
      kind: 'tale',
      status,
      // ONLY the title is mirrored, and only because the table requires both
      // titles to be non-empty. The description and the body are not: copying
      // them would put Russian prose on the English page — and, worse, the
      // copy used to land back in the editor, so writing in one language made
      // text appear in the other while the author was still typing.
      titleEn: copyOr(titleEn, titleRu),
      titleRu: copyOr(titleRu, titleEn),
      dekEn: dekEn.trim() || null,
      dekRu: dekRu.trim() || null,
      bodyEn: bodyEn.trim() || null,
      bodyRu: bodyRu.trim() || null,
      figurineId: figurineId || null,
      // Пустое поле — это «нет подписи», и оно обязано доехать до сервера:
      // пропущенное поле там значит то же самое, но молча.
      author: author.trim() || null,
      imageUrl: imageUrls[0] ?? null,
      // An empty ARRAY, never null: `image_urls` is a plain `Vec<String>` with
      // a serde default on the server, and `default` covers a missing field,
      // not an explicit null — sending null is a 422. An empty array is also
      // what clears a photograph the keeper has removed.
      imageUrls,
      pinned,
      scheduledAt,
    };
  }

  /**
   * Quiet save. Never changes the status — a draft stays a draft.
   *
   * Deliberately does NOT read the saved leaf back into the editor. A save
   * fires while the author is mid-sentence, and the response arrives a moment
   * later carrying the text as it was when the request left: applying it threw
   * away everything typed in between and threw the caret to the end. Only the
   * identity the server owns is taken back.
   */
  async function save(): Promise<boolean> {
    if (!titleEn.trim() && !titleRu.trim()) return false;
    const wasNew = selectedId === null;
    const sent = fieldsKey();
    saving = true;
    try {
      const saved = await api.adminSaveGazetteLeaf(payload(), selectedId ?? undefined);
      const stillTyping = fieldsKey() !== sent;
      selectedId = saved.id;
      slug = saved.slug;
      updatedAt = saved.updatedAt;
      savedAt = new Date();
      // Clean only if nothing was typed while the request was in flight;
      // otherwise stay dirty and let the next autosave carry the rest.
      if (!stillTyping) {
        snapshot = fieldsKey();
        localStorage.removeItem(draftKey());
      }
      // The list only needs re-reading when a row appears or its label moves.
      if (wasNew) {
        await loadTales();
      } else {
        tales = tales.map((tale) =>
          tale.id === saved.id
            ? { ...tale, titleEn: saved.titleEn, titleRu: saved.titleRu, bodyEn: saved.bodyEn, bodyRu: saved.bodyRu, status: saved.status, pinned: saved.pinned, figurineName: saved.figurineName }
            : tale,
        );
      }
      return true;
    } catch (e) {
      flash(String(e), 6000);
      return false;
    } finally {
      saving = false;
    }
  }

  /** Замечания перед выходом. Не запрещают — называют. */
  function releaseWarnings(): string[] {
    const out: string[] = [];
    for (const code of ['en', 'ru'] as Lang[]) {
      const other = code === 'en' ? 'ru' : 'en';
      if (bodyMissing[code]) {
        out.push($t('adminTalesCheckOneLang').replace('{lang}', code.toUpperCase()).replace('{other}', other.toUpperCase()));
      } else if (!(code === 'en' ? titleEn : titleRu).trim()) {
        out.push($t('adminTalesCheckNoTitle').replace('{lang}', code.toUpperCase()).replace('{other}', other.toUpperCase()));
      }
    }
    if (!plate) out.push($t('adminTalesCheckNoPhoto'));
    return out;
  }

  /**
   * Первый шаг выхода: сохранить (у новой байки ещё нет id, по которому
   * спросить число писем), спросить сервер и показать листок. Статус не
   * меняется, пока листок не подтвердили.
   */
  async function beginRelease(target: 'published' | 'scheduled', at: string | null) {
    if (!titleEn.trim() && !titleRu.trim()) {
      flash($t('adminTalesNeedTitle'));
      return;
    }
    // Пустое тело — не замечание, а отказ: письмо «вышла новая байка» о
    // байке без единой строки уйти не должно.
    if (!bodyEn.trim() && !bodyRu.trim()) {
      flash($t('adminTalesCheckNoBody'));
      return;
    }
    preflightBusy = true;
    try {
      if (autosaveTimer) clearTimeout(autosaveTimer);
      if (!(await save()) || !selectedId) return;
      let forecast: TaleLettersForecast | null = null;
      try {
        forecast = await api.adminTaleLettersForecast(selectedId);
      } catch {
        forecast = null;
      }
      preflight = { target, at, warnings: releaseWarnings(), forecast };
    } finally {
      preflightBusy = false;
    }
  }

  async function confirmRelease() {
    const p = preflight;
    if (!p) return;
    preflight = null;
    scheduling = false;
    status = p.target;
    if (p.target === 'scheduled') scheduledAt = p.at;
    if (await save()) {
      await loadTales();
      if (p.target === 'published') {
        sealed = true;
        if (sealTimer) clearTimeout(sealTimer);
        sealTimer = setTimeout(() => (sealed = false), 2600);
      }
    }
  }

  function cancelRelease() {
    preflight = null;
  }

  function publish() {
    void beginRelease('published', null);
  }

  async function setStatus(next: GazetteStatus) {
    // Выход на люди идёт только через листок: кнопка статуса «Опубликовано»
    // была обходом той самой проверки, ради которой листок заведён.
    if (next === 'published') return publish();
    // «Запланировано» без даты — не статус, а намерение: байка остаётся
    // прежней, пока не выбрана дата, и тогда проходит тот же листок.
    if (next === 'scheduled') {
      scheduling = true;
      return;
    }
    scheduling = false;
    status = next;
    if (await save()) await loadTales();
  }

  function pickSchedule(value: string) {
    if (!value) return;
    void beginRelease('scheduled', new Date(value).toISOString());
  }

  /** «Через 10 минут», «через 10 минут после 3 окт., 12:00». */
  function whenLetters(p: NonNullable<typeof preflight>): string {
    const mins = Math.round((p.forecast?.graceSecs ?? 600) / 60);
    if (p.target === 'scheduled' && p.at) {
      const at = new Date(p.at).toLocaleString($lang === 'ru' ? 'ru-RU' : 'en-GB', {
        day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit',
      });
      return $t('adminTalesLettersAfter').replace('{min}', String(mins)).replace('{at}', at);
    }
    return $t('adminTalesLettersIn').replace('{min}', String(mins));
  }

  async function destroy() {
    if (!selectedId) return;
    if (!confirm($t('adminTalesDeleteConfirm'))) return;
    try {
      await api.adminDeleteGazetteLeaf(selectedId);
    } catch (e) {
      flash(String(e), 6000);
      return;
    }
    localStorage.removeItem(draftKey());
    blank();
    await loadTales();
  }

  // Autosave. The effect reads `key` and `snapshot`; writing `snapshot` in save()
  // re-runs it once, finds nothing changed, and stops — no loop.
  $effect(() => {
    const now = key;
    const mark = snapshot;
    if (!ready || now === mark) return;
    try {
      localStorage.setItem(draftKey(), JSON.stringify({ ...JSON.parse(now), at: new Date().toISOString() }));
    } catch {
      // A full or blocked store must never stop the writing.
    }
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => {
      if (autosaveTimer) clearTimeout(autosaveTimer);
    };
  });

  // ── The paper's own gestures ───────────────────────────────────────────────

  function insertOrnament() {
    const box = bodyBox;
    if (!box) return;
    const at = box.selectionStart ?? box.value.length;
    const end = box.selectionEnd ?? at;
    const before = box.value.slice(0, at).replace(/\n+$/, '');
    const after = box.value.slice(end).replace(/^\n+/, '');
    const next = `${before}\n\n${ORNAMENT}\n\n${after}`;
    if (editLang === 'ru') bodyRu = next;
    else bodyEn = next;
    const caret = before.length + ORNAMENT.length + 4;
    void tick().then(() => {
      box.focus();
      box.setSelectionRange(caret, caret);
    });
  }

  function deskKeys(event: KeyboardEvent) {
    const meta = event.metaKey || event.ctrlKey;
    if (meta && event.key.toLowerCase() === 's') {
      event.preventDefault();
      if (autosaveTimer) clearTimeout(autosaveTimer);
      void save();
    } else if (meta && event.key === 'Enter') {
      event.preventDefault();
      insertOrnament();
    } else if (event.key === 'Escape' && focus) {
      focus = false;
    }
  }

  /** Two columns of the same tale, kept level so a translator can follow. */
  function mirrorScroll(from: HTMLTextAreaElement | null, to: HTMLTextAreaElement | null) {
    if (!both || syncing || !from || !to) return;
    const room = from.scrollHeight - from.clientHeight;
    if (room <= 0) return;
    syncing = true;
    to.scrollTop = (from.scrollTop / room) * (to.scrollHeight - to.clientHeight);
    requestAnimationFrame(() => (syncing = false));
  }

  // ── The shelf ──────────────────────────────────────────────────────────────

  function onDrop(to: number) {
    const from = dragFrom;
    dragFrom = null;
    dragOver = null;
    if (from == null || from === to) return;
    const next = [...tales];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    tales = next;
    if (reorderTimer) clearTimeout(reorderTimer);
    reorderTimer = setTimeout(async () => {
      try {
        await api.adminReorderTales(tales.map((tale) => tale.id));
        flash($t('adminTalesReordered'));
      } catch (e) {
        flash(String(e), 6000);
        await loadTales();
      }
    }, REORDER_MS);
  }

  function pickFrame(url: string) {
    imageUrls = [url];
  }

  /** The work's full photo set, fetched only once a work is on the desk. */
  async function loadFrames(id: string) {
    try {
      extraFig = await api.getFigurine(id);
    } catch {
      extraFig = null;
    }
  }

  $effect(() => {
    const id = figurineId;
    if (!id) {
      extraFig = null;
      return;
    }
    void loadFrames(id);
  });

  async function uploadPhoto() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      uploading = true;
      try {
        const imported = await api.importMediaWithVariants(file, 'images', titleEn || titleRu || 'tale');
        imageUrls = [imported.url];
      } catch (e) {
        flash(String(e), 6000);
      } finally {
        uploading = false;
      }
    };
    input.click();
  }

  /**
   * ISO-время в значение `datetime-local`. Срез `slice(0, 16)` давал время по
   * Гринвичу, а поле читает его как местное: запланированная на 12:00 байка
   * показывалась на 9:00 и при следующей правке уезжала на три часа.
   */
  function localInput(iso: string): string {
    const d = new Date(iso);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function clock(at: Date): string {
    return at.toLocaleTimeString($lang === 'ru' ? 'ru-RU' : 'en-GB', {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function titleOf(tale: GazetteLeaf): string {
    return (editLang === 'ru' ? tale.titleRu : tale.titleEn) || tale.titleEn || tale.titleRu || $t('adminTalesUntitled');
  }

  const STATUS_TONE: Record<GazetteStatus, string> = {
    draft: 'bg-[#b9a68f]',
    scheduled: 'bg-amber-500',
    published: 'bg-emerald-600',
    archived: 'bg-[#b0a08e]',
  };

  onMount(async () => {
    try {
      const [_, figs] = await Promise.all([loadTales(), api.getAllFigurines(), loadStats()]);
      figurines = figs;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      loading = false;
      blank();
      ready = true;
    }
  });

  onDestroy(() => {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    if (reorderTimer) clearTimeout(reorderTimer);
    if (sealTimer) clearTimeout(sealTimer);
  });

  // Arriving from a work's form: the desk opens with that work already pinned.
  $effect(() => {
    if (!seed || !ready) return;
    const s = seed;
    onSeedConsumed?.();
    if (s.leafId) {
      const existing = tales.find((tale) => tale.id === s.leafId);
      if (existing) {
        openTale(existing);
        return;
      }
    }
    blank();
    if (s.figurineId) figurineId = s.figurineId;
    if (s.imageUrls?.length) imageUrls = [...s.imageUrls];
  });
</script>

<svelte:window on:keydown={deskKeys} />

<div class="h-full flex text-[#34251c]" class:focus-mode={focus}>
  <!-- ── The shelf ──────────────────────────────────────────────────────── -->
  <aside class="shelf-pane w-[260px] flex-shrink-0 border-r border-[#34251c]/10 flex flex-col">
    <div class="p-3 border-b border-[#34251c]/10 space-y-2">
      <button
        onclick={startNew}
        class="w-full py-2 text-[10px] uppercase tracking-[0.18em] border border-[#c65f3c]/40 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors"
      >{$t('adminTalesNew')}</button>
      <button
        onclick={() => (pollDesk = !pollDesk)}
        class="w-full py-1.5 text-[10px] uppercase tracking-[0.16em] border transition-colors
          {pollDesk ? 'border-[#34251c]/40 bg-[#34251c]/5 text-[#34251c]' : 'border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c]'}"
      >{$t('adminTalesPollButton')}</button>
      <input
        bind:value={listQuery}
        placeholder={$t('adminTalesSearch')}
        class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 focus:border-[#34251c]/35 outline-none"
      />
    </div>

    <div class="flex-1 overflow-y-auto">
      {#if loading}
        <p class="p-3 text-xs text-[#5f4636]">…</p>
      {:else if visible.length === 0}
        <p class="p-3 text-xs italic text-[#5f4636]">{$t('adminTalesEmpty')}</p>
      {:else}
        <p class="px-3 py-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminTalesDragHint')}</p>
        <ul class="pb-4">
          {#each visible as tale, i (tale.id)}
            <li
              draggable={!listQuery}
              ondragstart={() => (dragFrom = i)}
              ondragover={(e) => { e.preventDefault(); dragOver = i; }}
              ondragleave={() => { if (dragOver === i) dragOver = null; }}
              ondrop={(e) => { e.preventDefault(); onDrop(i); }}
              ondragend={() => { dragFrom = null; dragOver = null; }}
              class="border-b border-[#34251c]/5 {dragOver === i ? 'bg-[#c65f3c]/10' : ''} {dragFrom === i ? 'opacity-40' : ''}"
            >
              <button
                onclick={() => openTale(tale)}
                class="w-full text-left px-3 py-2.5 flex gap-2 items-start hover:bg-[#34251c]/[0.04] transition-colors
                  {selectedId === tale.id ? 'bg-[#34251c]/[0.06]' : ''}"
              >
                <span class="mt-1.5 w-1.5 h-1.5 rounded-full flex-shrink-0 {STATUS_TONE[tale.status]}"></span>
                <span class="min-w-0">
                  <span class="block text-[13px] leading-snug truncate" style="font-family: 'Cormorant Garamond', Georgia, serif;">
                    {titleOf(tale)}
                  </span>
                  {#if tale.figurineName}
                    <span class="block text-[10px] text-[#8a6a55] truncate">{tale.figurineName}</span>
                  {/if}
                  <!-- Половина сайта без текста видна в списке, а не только
                       на открытой байке: иначе её находят читатели. Пишется
                       только недостающее — «EN RU» у каждой строки было бы шумом. -->
                  {#if (tale.bodyEn ?? '').trim() && !(tale.bodyRu ?? '').trim()}
                    <span class="lang-gap">{$t('adminTalesLangGap').replace('{lang}', 'RU')}</span>
                  {:else if (tale.bodyRu ?? '').trim() && !(tale.bodyEn ?? '').trim()}
                    <span class="lang-gap">{$t('adminTalesLangGap').replace('{lang}', 'EN')}</span>
                  {/if}
                </span>
                {#if (stats[tale.id]?.pendingComments ?? 0) > 0}
                  <span class="ml-auto text-[#c65f3c] text-[10px]" title={$t('adminTalesPending')}>
                    {stats[tale.id].pendingComments}✍
                  </span>
                {/if}
                <!-- Сколько ждут продолжения — видно в списке, а не только на
                     открытой байке: это ответ на вопрос «что писать дальше». -->
                {#if (stats[tale.id]?.sequelWishes ?? 0) > 0}
                  <span class="ml-auto text-[#6f3b24] text-[10px] whitespace-nowrap" title={$t('adminTalesSequelWishes')}>
                    +{stats[tale.id].sequelWishes}
                  </span>
                {/if}
                {#if tale.pinned}<span class="ml-auto text-[#c65f3c] text-[10px]">◆</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </aside>

  <!-- ── The desk ───────────────────────────────────────────────────────── -->
  <div class="flex-1 flex flex-col min-w-0 bg-[#f8f1e7]">
    {#if pollDesk}
      <TalePollDesk {figurines} {tales} onWrite={writeChosen} onBack={() => (pollDesk = false)} />
    {:else}
    <div class="desk-chrome flex items-center gap-3 px-4 py-2 border-b border-[#34251c]/10 text-[10px] uppercase tracking-[0.16em]">
      <div class="flex border border-[#34251c]/15">
        {#each ['en', 'ru'] as code}
          <button
            onclick={() => (editLang = code as Lang)}
            class="px-2 py-1 transition-colors {editLang === code ? 'bg-[#34251c]/10 text-[#34251c]' : 'text-[#8a6a55]'}"
            title={bodyMissing[code as Lang] ? $t('adminTalesLangEmpty') : undefined}
          >{code}{#if bodyMissing[code as Lang] && open}<span class="lang-hollow" aria-label={$t('adminTalesLangEmpty')}>○</span>{/if}</button>
        {/each}
      </div>
      <button
        onclick={() => (both = !both)}
        class="px-2 py-1 border transition-colors {both ? 'border-[#c65f3c]/50 text-[#c65f3c]' : 'border-[#34251c]/15 text-[#8a6a55]'}"
      >{$t('adminTalesBoth')}</button>
      <button
        onclick={insertOrnament}
        class="px-2 py-1 border border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c] transition-colors"
        title={$t('adminTalesOrnament')}
        aria-label={$t('adminTalesOrnament')}
      >{ORNAMENT}</button>
      <button
        onclick={() => (focus = !focus)}
        class="px-2 py-1 border border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c] transition-colors"
      >{focus ? $t('adminTalesFocusExit') : $t('adminTalesFocus')}</button>
      <button
        onclick={() => (markupOpen = !markupOpen)}
        class="px-2 py-1 border transition-colors {markupOpen ? 'border-[#34251c]/40 text-[#34251c]' : 'border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c]'}"
        aria-expanded={markupOpen}
      >{$t('adminTalesMarkup')}</button>
      <button
        onclick={() => (reading = !reading)}
        class="px-2 py-1 border transition-colors {reading ? 'border-[#c65f3c]/50 text-[#c65f3c]' : 'border-[#34251c]/15 text-[#8a6a55] hover:text-[#34251c]'}"
        aria-pressed={reading}
      >{reading ? $t('adminTalesReadingOff') : $t('adminTalesReadingOn')}</button>

      <!-- Где байка живёт на сайте. Ссылка — только у той, что на людях:
           у черновика адрес отвечает «не найдено», и ссылка туда была бы
           обещанием, которое страница не сдержит. -->
      {#if siteHref}
        {#if status === 'published'}
          <a href={siteHref} target="_blank" rel="noopener" class="normal-case tracking-normal text-[11px] text-[#c65f3c] hover:underline">{siteHref} ↗</a>
        {:else if slugEditable}
          <label class="normal-case tracking-normal text-[11px] text-[#8a6a55] flex items-center" title={$t('adminTalesSlugHint')}>
            /tales/<input
              bind:value={slugDraft}
              onblur={commitSlug}
              onkeydown={(e) => { if (e.key === 'Enter') e.currentTarget.blur(); else if (e.key === 'Escape') { slugDraft = slug; e.currentTarget.blur(); } }}
              maxlength="120"
              aria-label={$t('adminTalesSlugHint')}
              class="slug-input w-[16ch] px-1 py-0.5 bg-transparent border-b border-[#34251c]/20 focus:border-[#34251c]/50 outline-none text-[#34251c]"
            />
          </label>
        {:else}
          <span class="normal-case tracking-normal text-[11px] text-[#8a6a55]" title={addressFixed ? $t('adminTalesSlugFixed') : $t('adminTalesSiteHidden')}>{siteHref}</span>
        {/if}
      {/if}

      <span class="ml-auto normal-case tracking-normal text-[11px] text-[#8a6a55]">
        {#if saving}{$t('adminTalesSaving')}
        {:else if savedAt}{$t('adminTalesSaved').replace('{time}', clock(savedAt))}
        {:else if dirty}{$t('adminTalesUnsaved')}{/if}
      </span>
    </div>

    {#if markupOpen}
      <!-- Что понимает тело байки. Разметки ровно столько, сколько знает
           renderTale; остальное печатается как написано. -->
      <div class="desk-chrome markup px-4 py-3 border-b border-[#34251c]/10">
        <dl>
          <dt># {$t('adminTalesMarkupHeadEx')}</dt><dd>{$t('adminTalesMarkupHead')}</dd>
          <dt>**{$t('adminTalesMarkupBoldEx')}**</dt><dd>{$t('adminTalesMarkupBold')}</dd>
          <dt>{ORNAMENT}</dt><dd>{$t('adminTalesMarkupOrnament')}</dd>
          <dt>ritunia.com/figurines/…</dt><dd>{$t('adminTalesMarkupLink')}</dd>
        </dl>
      </div>
    {/if}

    <div class="flex-1 overflow-y-auto">
      <div class="desk mx-auto px-6 py-8">
        {#if restorable}
          <div class="mb-6 px-3 py-2 border border-[#c65f3c]/35 bg-[#c65f3c]/5 text-xs flex items-center gap-3">
            <span class="flex-1">{$t('adminTalesRestore')}</span>
            <button onclick={restoreDraft} class="uppercase tracking-[0.14em] text-[10px] text-[#c65f3c]">{$t('adminTalesRestoreDo')}</button>
            <button onclick={dropDraft} class="uppercase tracking-[0.14em] text-[10px] text-[#8a6a55]">{$t('adminTalesRestoreDrop')}</button>
          </div>
        {/if}

        <div class="paper">
          <input
            bind:value={
              () => titleNow,
              (v) => { if (editLang === 'ru') titleRu = v; else titleEn = v; }
            }
            maxlength={TITLE_MAX}
            placeholder={$t('adminTalesTitlePh')}
            class="paper-title"
          />
          <input
            bind:value={
              () => dekNow,
              (v) => { if (editLang === 'ru') dekRu = v; else dekEn = v; }
            }
            maxlength={DEK_MAX}
            placeholder={$t('adminTalesEpigraphPh')}
            class="paper-epigraph"
          />
          <!-- Подпись под историей. Стоит там же, где потом напечатается:
               под заглавием, а не в отдельной форме внизу. Пусто — подписи
               на странице не будет. -->
          <input
            bind:value={author}
            maxlength={AUTHOR_MAX}
            placeholder={$t('adminTalesAuthorPh')}
            class="paper-byline"
          />

          {#if reading}
            <div class="columns" class:two={both}>
              <div class="reading">
                <TaleProse blocks={renderTale(bodyNow)} links={readingLinks} />
              </div>
              {#if both}
                <div class="reading second">
                  <TaleProse blocks={renderTale(otherBody)} links={readingLinks} />
                </div>
              {/if}
            </div>
          {:else}
          <div class="columns" class:two={both}>
            <textarea
              bind:this={bodyBox}
              bind:value={
                () => bodyNow,
                (v) => { if (editLang === 'ru') bodyRu = v; else bodyEn = v; }
              }
              onscroll={() => mirrorScroll(bodyBox, secondBox)}
              maxlength={BODY_MAX}
              placeholder={$t('adminTalesBodyPh')}
              class="paper-body"
            ></textarea>

            {#if both}
              <textarea
                bind:this={secondBox}
                bind:value={
                  () => otherBody,
                  (v) => { if (otherLang === 'ru') bodyRu = v; else bodyEn = v; }
                }
                onscroll={() => mirrorScroll(secondBox, bodyBox)}
                maxlength={BODY_MAX}
                placeholder={otherTitle || otherLang.toUpperCase()}
                class="paper-body second"
              ></textarea>
            {/if}
          </div>
          {/if}

          <!-- Length as a shadow, not a number: a rising hairline that turns
               copper once the little story has stopped being little. -->
          <div class="measure-row">
            <span class="measure-label" class:over={tooLong}>
              {tooLong ? $t('adminTalesTooLong') : $t('adminTalesLength')} — {chars}{#if minutes} · {$t('adminTalesReading').replace('{n}', String(minutes)).replace('{unit}', $t(minutesKey(minutes)))}{/if}
            </span>
            <span class="measure">
              <span class="measure-fill" class:over={tooLong} style="width: {fill * 100}%"></span>
            </span>
          </div>
        </div>

        <!-- Отклик на открытую байку: числа и очередь модерации. Стоит под
             бумагой, а не в подвальной ленте: лента — про то, что с байкой
             делают, а это про то, что с ней уже случилось. Минус печатается
             только здесь — читателю его не показывают. -->
        {#if selectedId}
          <section class="talk">
            <div class="talk-nums">
              <span class="num"><b>{mine?.views ?? 0}</b> {$t('adminTalesViews')}</span>
              <span class="num"><b>{mine?.likes ?? 0}</b> {$t('adminTalesLikes')}</span>
              <span class="num num--ill"><b>{mine?.dislikes ?? 0}</b> {$t('adminTalesDislikes')}</span>
              <span class="num"><b>{mine?.comments ?? 0}</b> {$t('adminTalesComments')}</span>
              {#if (mine?.sequelWishes ?? 0) > 0}
                <span class="num num--wait">
                  {$t('adminTalesSequelWishes')}: <b>{mine?.sequelWishes}</b>{#if (mine?.sequelLetters ?? 0) > 0}, {$t('adminTalesSequelLetters')}: <b>{mine?.sequelLetters}</b>{/if}
                </span>
              {/if}
              {#if (mine?.pendingComments ?? 0) > 0}
                <span class="num num--wait"><b>{mine?.pendingComments}</b> {$t('adminTalesPending')}</span>
              {/if}
              {#if (mine?.sequelWishes ?? 0) > 0}
                <button class="talk-toggle" onclick={toggleWaiting}>
                  {waitingOpen ? $t('adminTalesWaitingHide') : $t('adminTalesWaitingShow')}
                </button>
              {/if}
              <button class="talk-toggle talk-toggle--next" onclick={() => (talkOpen = !talkOpen)}>
                {talkOpen ? $t('adminTalesTalkHide') : $t('adminTalesTalkShow')}
              </button>
            </div>

            <!-- Кто ждёт продолжения и дошло ли до него известие. Письма и
                 записки уходят сами, когда у новой байки выбрано «Продолжает»;
                 здесь их только видно. -->
            {#if waitingOpen}
              <div class="waiting">
                <p class="waiting-hint">{$t('adminTalesWaitingHint')}</p>
                {#if waitingLoading}
                  <p class="talk-empty">…</p>
                {:else if waiting.length === 0}
                  <p class="talk-empty">{$t('adminTalesWaitingEmpty')}</p>
                {:else}
                  <ul class="talk-list">
                    {#each waiting as w, i (i)}
                      <li class="talk-item">
                        <div class="talk-head">
                          <span class="talk-who">{w.name ?? w.email ?? $t('adminTalesWaitingGuest')}</span>
                          {#if w.email && w.name}<span class="talk-mail">{w.email}</span>{/if}
                          {#if w.telegramUsername}<span class="talk-mail">@{w.telegramUsername}</span>{/if}
                          <span class="talk-when">{new Date(w.createdAt).toLocaleDateString()}</span>
                        </div>
                        <p class="waiting-ways">
                          {#if w.email}
                            <span class="way way--{w.letter}">{$t('adminTalesWaitingLetter')}: {$t(NOTICE_WORD[w.letter] as never)}</span>
                          {/if}
                          {#if w.byTelegram}
                            <span class="way way--{w.note}">Telegram: {$t(NOTICE_WORD[w.note] as never)}</span>
                          {/if}
                          {#if !w.email && !w.byTelegram}
                            <span class="way way--none">{$t('adminTalesWaitingNoWay')}</span>
                          {/if}
                        </p>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}

            {#if talkOpen}
              {#if commentsLoading}
                <p class="talk-empty">…</p>
              {:else if comments.length === 0}
                <p class="talk-empty">{$t('adminTalesNoComments')}</p>
              {:else}
                <ul class="talk-list">
                  {#each comments as c (c.id)}
                    <li class="talk-item" class:waiting={!c.isApproved}>
                      <div class="talk-head">
                        <span class="talk-who">{c.authorName}</span>
                        {#if c.authorEmail}<span class="talk-mail">{c.authorEmail}</span>{/if}
                        <span class="talk-when">{new Date(c.createdAt).toLocaleString()}</span>
                        {#if !c.isApproved}<span class="talk-flag">{$t('adminTalesPending')}</span>{/if}
                      </div>
                      <p class="talk-body">{c.body}</p>
                      {#if c.adminReply}
                        <p class="talk-reply"><span>{$t('adminTalesReplyLabel')}</span> {c.adminReply}</p>
                      {/if}

                      {#if replyFor === c.id}
                        <div class="talk-reply-box">
                          <textarea
                            bind:value={replyText}
                            rows="3"
                            placeholder={$t('adminTalesReplyPh')}
                            class="talk-reply-input"
                          ></textarea>
                          <div class="talk-acts">
                            <button class="act act--do" onclick={() => moderate(c, true, replyText)}>
                              {$t('adminTalesReplySend')}
                            </button>
                            <button class="act" onclick={() => { replyFor = null; replyText = ''; }}>
                              {$t('adminTalesCancel')}
                            </button>
                          </div>
                        </div>
                      {:else}
                        <div class="talk-acts">
                          {#if c.isApproved}
                            <button class="act" onclick={() => moderate(c, false)}>{$t('adminTalesHide')}</button>
                          {:else}
                            <button class="act act--do" onclick={() => moderate(c, true)}>{$t('adminTalesApprove')}</button>
                          {/if}
                          <button
                            class="act"
                            onclick={() => { replyFor = c.id; replyText = c.adminReply ?? ''; }}
                          >{$t('adminTalesReply')}</button>
                          <button class="act act--ill" onclick={() => removeComment(c)}>{$t('adminTalesDelete')}</button>
                        </div>
                      {/if}
                    </li>
                  {/each}
                </ul>
              {/if}
            {/if}
          </section>
        {/if}
      </div>
    </div>

    {#if preflight}
      {@const p = preflight}
      <!-- Листок перед выходом. Стоит над лентой, у той самой кнопки, а не
           окном поверх бумаги: байку под ним видно, и замечание можно
           проверить глазами, не закрывая его. -->
      <div class="preflight desk-chrome border-t border-[#c65f3c]/30 px-4 py-3 text-xs" role="alertdialog" aria-label={$t('adminTalesPreflight')}>
        <p class="pf-head">{p.target === 'scheduled' ? $t('adminTalesPreflightScheduled') : $t('adminTalesPreflight')}</p>
        {#if p.warnings.length}
          <ul class="pf-warn">
            {#each p.warnings as w (w)}<li>{w}</li>{/each}
          </ul>
        {/if}
        <p class="pf-letters">
          {#if !p.forecast}
            {$t('adminTalesLettersUnknown')}
          {:else if p.forecast.laid}
            {$t('adminTalesLettersLaid')}
          {:else if p.forecast.letters === 0 && p.forecast.notes === 0}
            {$t('adminTalesLettersNone')}
          {:else}
            <!-- «писем: 5», а не «5 писем»: так число не спорит с падежом. -->
            {whenLetters(p)} —
            {#if p.forecast.letters > 0}{$t('adminTalesLettersMail')}: <b>{p.forecast.letters}</b>{/if}{#if p.forecast.letters > 0 && p.forecast.notes > 0}, {/if}{#if p.forecast.notes > 0}{$t('adminTalesLettersNotes')}: <b>{p.forecast.notes}</b>{/if}.
            {$t('adminTalesLettersUndo')}
            {#if !p.forecast.mail && p.forecast.letters > 0}
              <span class="pf-nomail">{$t('adminTalesLettersNoMail')}</span>
            {/if}
          {/if}
        </p>
        <div class="flex gap-3 mt-2">
          <button
            onclick={confirmRelease}
            class="px-3 py-1.5 text-[10px] uppercase tracking-[0.18em] border border-[#c65f3c]/50 text-[#c65f3c] hover:bg-[#c65f3c]/5"
          >{p.target === 'scheduled' ? $t('adminTalesPreflightSchedule') : $t('adminTalesPreflightGo')}</button>
          <button onclick={cancelRelease} class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#34251c]">
            {$t('adminTalesPreflightBack')}
          </button>
        </div>
      </div>
    {/if}

    <!-- ── The margin: the work, the status, the seal ──────────────────── -->
    <div class="desk-chrome border-t border-[#34251c]/10 px-4 py-3 flex items-start gap-5 text-xs">
      <div class="w-[220px] flex-shrink-0">
        <p class="text-[9px] uppercase tracking-[0.16em] text-[#8a6a55] mb-1.5">{$t('adminTalesWork')}</p>
        {#if selectedFig}
          <div class="flex gap-2 items-start">
            {#if plate}
              <img src={plate} alt="" class="w-12 h-14 object-cover border border-[#d8c6b1] bg-[#1a120e]" />
            {/if}
            <div class="min-w-0">
              <p class="truncate">{selectedFig.name}</p>
              <div class="flex flex-wrap gap-2 mt-1 text-[10px]">
                <button onclick={() => { figurineId = ''; }} class="text-[#8a6a55] hover:text-[#34251c]">{$t('adminTalesWorkClear')}</button>
                <button onclick={uploadPhoto} class="text-[#8a6a55] hover:text-[#34251c]" disabled={uploading}>
                  {uploading ? '…' : $t('adminTalesUpload')}
                </button>
                {#if plate}
                  <button onclick={() => (imageUrls = [])} class="text-[#8a6a55] hover:text-[#34251c]">{$t('adminTalesDropPhoto')}</button>
                {/if}
              </div>
              {#if frames.length > 0}
                <div class="flex flex-wrap gap-1.5 mt-2">
                  {#each frames as url (url)}
                    <button
                      type="button"
                      onclick={() => pickFrame(url)}
                      title={$t('adminTalesUseWorkPhoto')}
                      class="w-8 h-10 p-0 overflow-hidden border {plate === url ? 'border-[#c65f3c]' : 'border-[#d8c6b1] hover:border-[#8a6a55]'}"
                    >
                      <img src={url} alt="" class="w-full h-full object-cover" />
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <div class="relative">
            <input
              bind:value={figQuery}
              onfocus={() => (figOpen = true)}
              onblur={() => setTimeout(() => (figOpen = false), 150)}
              placeholder={$t('adminTalesWorkSearch')}
              class="w-full px-2 py-1 bg-transparent border border-[#34251c]/15 focus:border-[#34251c]/35 outline-none text-xs"
            />
            {#if figOpen && figMatches.length}
              <ul class="absolute bottom-full left-0 right-0 mb-1 max-h-56 overflow-y-auto bg-[#f8f1e7] border border-[#34251c]/15 shadow-lg z-20">
                {#each figMatches as fig (fig.id)}
                  <li>
                    <button
                      onclick={() => { figurineId = fig.id; figQuery = ''; figOpen = false; }}
                      class="w-full text-left px-2 py-1.5 hover:bg-[#34251c]/5 truncate"
                    >{fig.name}</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex-1 min-w-0">
        <p class="text-[9px] uppercase tracking-[0.16em] text-[#8a6a55] mb-1.5">{$t('adminTalesState')}</p>
        <div class="flex flex-wrap items-center gap-2">
          {#each [['draft', 'adminTalesStatusDraft'], ['scheduled', 'adminTalesStatusScheduled'], ['published', 'adminTalesStatusPublished'], ['archived', 'adminTalesStatusArchived']] as [value, label]}
            <button
              onclick={() => setStatus(value as GazetteStatus)}
              disabled={!open}
              class="px-2 py-1 text-[10px] uppercase tracking-[0.14em] border transition-colors
                {status === value || (value === 'scheduled' && scheduling) ? 'border-[#34251c]/40 bg-[#34251c]/5 text-[#34251c]' : 'border-[#34251c]/12 text-[#8a6a55] hover:text-[#34251c]'}"
            >{$t(label as never)}</button>
          {/each}

          {#if status === 'scheduled' || scheduling}
            <!-- Дата уходит через тот же листок, что и «Опубликовать»: в
                 назначенный час байка разойдётся письмами так же. -->
            <input
              type="datetime-local"
              value={scheduledAt ? localInput(scheduledAt) : ''}
              onchange={(e) => pickSchedule(e.currentTarget.value)}
              class="px-2 py-1 bg-transparent border border-[#34251c]/15 text-[11px]"
            />
            {#if scheduling && status !== 'scheduled'}
              <span class="text-[10px] text-[#8a6a55]">{$t('adminTalesPickDate')}</span>
            {/if}
          {/if}

          <label class="flex items-center gap-1.5 text-[10px] text-[#5f4636] ml-2">
            <input type="checkbox" bind:checked={pinned} class="accent-[#c65f3c]" />
            {$t('adminTalesPinned')}
          </label>
        </div>
        <!-- Продолжение: кто просил продолжение начала, получит письмо, когда
             эта байка выйдет. Только у сохранённой байки — у несохранённой
             нет id, на который сослаться. -->
        {#if selectedId}
          <label class="mt-2 flex items-center gap-2 text-[10px] text-[#5f4636]">
            <span class="uppercase tracking-[0.14em] text-[#8a6a55]">{$t('adminTalesSequelOf')}</span>
            <select
              value={mine?.sequelOf ?? ''}
              onchange={(e) => setSequel(e.currentTarget)}
              class="max-w-[260px] px-1.5 py-1 bg-transparent border border-[#34251c]/15 text-[11px]"
            >
              <option value="">{$t('adminTalesSequelNone')}</option>
              {#each tales.filter((other) => other.id !== selectedId) as other (other.id)}
                <option value={other.id}>{titleOf(other)}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>

      <div class="flex-shrink-0 flex items-center gap-3">
        {#if sealed}
          <span class="seal">{$t('adminTalesPublishedSeal')}</span>
        {/if}
        <button
          onclick={publish}
          disabled={!open || saving || preflightBusy || preflight !== null}
          class="px-3 py-2 text-[10px] uppercase tracking-[0.18em] border border-[#c65f3c]/50 text-[#c65f3c] hover:bg-[#c65f3c]/5 transition-colors disabled:opacity-40"
        >{$t('adminTalesPublish')}</button>
        {#if selectedId}
          <button onclick={destroy} class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-red-700">
            {$t('adminTalesDelete')}
          </button>
        {/if}
      </div>
    </div>
    {/if}

    {#if message}
      <p class="px-4 py-2 text-xs border-t border-[#34251c]/10 text-[#6f3b24]">{message}</p>
    {/if}
  </div>
</div>

<style>
  /* The desk is the paper: same faces, same measure, same colour the room
     will read it in. Nothing here is a labelled field. */
  .desk { max-width: 780px; }

  .paper-title,
  .paper-epigraph,
  .paper-byline,
  .paper-body {
    display: block;
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: #34251c;
    font-family: 'Cormorant Garamond', Georgia, serif;
  }
  .paper-title::placeholder,
  .paper-epigraph::placeholder,
  .paper-byline::placeholder,
  .paper-body::placeholder { color: #b9a68f; }

  .paper-title {
    font-size: clamp(30px, 3.4vw, 46px);
    font-weight: 300;
    line-height: 1.04;
    letter-spacing: -0.012em;
    margin-bottom: 12px;
  }

  .paper-epigraph {
    font-size: 19px;
    font-weight: 300;
    font-style: italic;
    color: #5f4636;
    margin-bottom: 10px;
  }

  .paper-byline {
    font-size: 16px;
    color: #5f4636;
    margin-bottom: 26px;
  }

  .talk {
    margin-top: 36px;
    padding-top: 18px;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
  }
  .talk-nums {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 16px;
    font-size: 9px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }
  .num { display: inline-flex; align-items: baseline; gap: 5px; }
  .num b {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-weight: 400;
    letter-spacing: 0;
    color: #34251c;
  }
  .num--ill b { color: #8a6a55; }
  .num--wait b { color: #c65f3c; }
  .talk-toggle {
    margin-left: auto;
    font-size: 9px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #c65f3c;
    background: none;
    border: none;
    cursor: pointer;
  }

  /* Второй переключатель стоит за первым, а не отъезжает к краю сам. */
  .talk-toggle + .talk-toggle--next { margin-left: 0; }

  .waiting { margin-top: 14px; }
  .waiting-hint { margin: 0; font-size: 12px; line-height: 1.5; color: #5f4636; max-width: 60ch; }
  .waiting-ways {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin: 6px 0 0;
    font-size: 11px;
    color: #5f4636;
  }
  /* Судьба известия — формой отметки, а не только словом: ушедшее залито,
     не дошедшее отбито красным. */
  .way { padding: 1px 7px; border: 1px solid rgba(52, 37, 28, 0.15); }
  .way--sent { background: rgba(198, 95, 60, 0.12); border-color: rgba(198, 95, 60, 0.4); color: #6f3b24; }
  .way--failed { border-color: #8a2a2a; color: #8a2a2a; }
  .way--pending { border-style: dashed; }

  .talk-empty { margin: 16px 0 0; font-size: 12px; font-style: italic; color: #8a6a55; }
  .talk-list { list-style: none; margin: 16px 0 0; padding: 0; }
  .talk-item {
    padding: 12px 0;
    border-bottom: 1px solid rgba(52, 37, 28, 0.08);
  }
  /* Неодобренное отбито полосой слева, а не цветом текста: текст читают, а
     полосу видно, не читая. */
  .talk-item.waiting {
    border-left: 2px solid #c65f3c;
    padding-left: 10px;
  }
  .talk-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 10px;
    font-size: 10px;
    color: #8a6a55;
  }
  .talk-who { font-size: 12px; color: #34251c; }
  .talk-flag { color: #c65f3c; text-transform: uppercase; letter-spacing: 0.14em; font-size: 9px; }
  .talk-body { margin: 6px 0 0; font-size: 14px; line-height: 1.55; color: #34251c; }
  .talk-reply {
    margin: 6px 0 0;
    padding-left: 12px;
    border-left: 1px solid rgba(52, 37, 28, 0.15);
    font-size: 13px;
    color: #5f4636;
  }
  .talk-reply span { font-size: 9px; letter-spacing: 0.14em; text-transform: uppercase; color: #8a6a55; }
  .talk-reply-box { margin-top: 8px; }
  .talk-reply-input {
    width: 100%;
    padding: 8px;
    font-size: 13px;
    background: transparent;
    border: 1px solid rgba(52, 37, 28, 0.15);
    outline: none;
    color: #34251c;
  }
  .talk-reply-input:focus { border-color: rgba(52, 37, 28, 0.35); }
  .talk-acts { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 8px; }
  .act {
    font-size: 9px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: color 0.2s;
  }
  .act:hover { color: #34251c; }
  .act--do { color: #c65f3c; }
  .act--ill:hover { color: #a33; }

  .columns { display: grid; gap: 24px; }
  .columns.two { grid-template-columns: 1fr 1fr; }

  .paper-body {
    font-size: 19px;
    line-height: 1.72;
    min-height: 46vh;
    resize: vertical;
  }
  .paper-body.second { color: #6f5847; }

  /* Предпросмотр занимает место полей: высота та же, чтобы бумага не
     прыгала при переключении. */
  .reading { min-height: 46vh; }
  .reading.second { opacity: 0.8; }

  .measure-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 18px;
  }

  .measure-label {
    flex-shrink: 0;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #8a6a55;
  }
  .measure-label.over { color: #c65f3c; }

  .measure {
    flex: 1;
    height: 1px;
    background: rgba(52, 37, 28, 0.1);
  }
  .measure-fill {
    display: block;
    height: 1px;
    background: rgba(52, 37, 28, 0.35);
    transition: width 0.4s ease, background-color 0.4s ease;
  }
  .measure-fill.over { background: #c65f3c; }

  /* Wax, the way the house does it everywhere else. */
  .seal {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 8px 14px;
    border-radius: 999px;
    background: radial-gradient(circle at 32% 28%, #d0704a 0%, #a4462a 60%, #7d3520 100%);
    color: #fbeee2;
    font-size: 9px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    box-shadow: inset 0 1px 0 rgba(255, 220, 200, 0.4), 0 2px 6px rgba(52, 37, 28, 0.25);
    animation: press 0.5s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
  @keyframes press {
    from { transform: scale(1.5) rotate(-8deg); opacity: 0; }
    to { transform: scale(1) rotate(-2deg); opacity: 1; }
  }

  .lang-gap {
    display: inline-block;
    margin-top: 2px;
    font-size: 9px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #c65f3c;
  }
  .lang-hollow { margin-left: 3px; font-size: 8px; color: #c65f3c; }

  .markup dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 16px;
    margin: 0;
    max-width: 780px;
    font-size: 12px;
    color: #5f4636;
  }
  .markup dt { font-family: ui-monospace, Menlo, monospace; color: #34251c; }
  .markup dd { margin: 0; }

  .preflight { background: rgba(198, 95, 60, 0.04); }
  .pf-head { margin: 0 0 6px; font-size: 9px; letter-spacing: 0.16em; text-transform: uppercase; color: #6f3b24; }
  .pf-warn { margin: 0 0 6px; padding-left: 16px; color: #6f3b24; line-height: 1.5; }
  .pf-letters { margin: 0; color: #34251c; line-height: 1.5; max-width: 80ch; }
  .pf-letters b { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 16px; font-weight: 500; }
  .pf-nomail { display: block; margin-top: 4px; color: #8a2a2a; }

  /* Nothing but paper. */
  .focus-mode :global(.shelf-pane),
  .focus-mode :global(.desk-chrome) { display: none; }

  @media (prefers-reduced-motion: reduce) {
    .seal { animation: none; }
    .measure-fill { transition: none; }
  }
</style>
