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
  import TaleSceneEditor from './TaleSceneEditor.svelte';
  import TaleShelfPicker from './TaleShelfPicker.svelte';
  import WorkPicker from './WorkPicker.svelte';
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
  // Сцена на полях стоит вместо бумаги на всю ширину стола: у неё свой скролл.
  let sceneMode = $state(false);
  let sealed = $state(false);
  let uploading = $state(false);

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

  /** Строку перетащили в списке выбора: индексы — в полном порядке полки. */
  function moveTale(from: number, to: number) {
    if (from === to) return;
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

<div class="panel" class:focus-mode={focus}>
  {#if focus}
    <button class="focus-exit" onclick={() => (focus = false)}>{$t('adminTalesFocusExit')}</button>
  {/if}

  <!-- ── Шапка: какой рассказ открыт ───────────────────────────────────── -->
  <header class="bar desk-chrome">
    <TaleShelfPicker
      {tales}
      {stats}
      {selectedId}
      draftTitle={titleNow || otherTitle}
      lang={editLang}
      {loading}
      onopen={openTale}
      onmove={moveTale}
    />
    <button class="btn btn--accent" onclick={startNew}>+ {$t('adminTalesNew')}</button>
    <button class="btn" class:on={pollDesk} aria-pressed={pollDesk} onclick={() => (pollDesk = !pollDesk)}>
      {$t('adminTalesPollButton')}
    </button>
    <span class="grow"></span>
    <span class="saved" aria-live="polite">
      {#if saving}{$t('adminTalesSaving')}
      {:else if savedAt}{$t('adminTalesSaved').replace('{time}', clock(savedAt))}
      {:else if dirty}{$t('adminTalesUnsaved')}{/if}
    </span>
  </header>

  {#if pollDesk}
    <TalePollDesk {figurines} {tales} onWrite={writeChosen} onBack={() => (pollDesk = false)} />
  {:else}
    <!-- ── Инструменты письма ───────────────────────────────────────────── -->
    <div class="tools desk-chrome">
      <div class="seg" role="group" aria-label="Language">
        {#each ['en', 'ru'] as code}
          <button
            class:on={editLang === code}
            onclick={() => (editLang = code as Lang)}
            title={bodyMissing[code as Lang] ? $t('adminTalesLangEmpty') : undefined}
          >{code.toUpperCase()}{#if bodyMissing[code as Lang] && open}<span class="lang-hollow" aria-label={$t('adminTalesLangEmpty')}>○</span>{/if}</button>
        {/each}
      </div>
      <button class="tb" class:on={both} aria-pressed={both} onclick={() => (both = !both)}>{$t('adminTalesBoth')}</button>
      <button class="tb tb--icon" onclick={insertOrnament} title={$t('adminTalesOrnament')} aria-label={$t('adminTalesOrnament')}>{ORNAMENT}</button>
      <span class="sep" aria-hidden="true"></span>
      <button class="tb" class:on={markupOpen} aria-expanded={markupOpen} onclick={() => (markupOpen = !markupOpen)}>{$t('adminTalesMarkup')}</button>
      <button class="tb" class:on={reading} aria-pressed={reading} onclick={() => (reading = !reading)}>
        {reading ? $t('adminTalesReadingOff') : $t('adminTalesReadingOn')}
      </button>
      <button
        class="tb"
        class:on={sceneMode}
        aria-pressed={sceneMode}
        onclick={() => (sceneMode = !sceneMode)}
        disabled={!selectedId}
        title={selectedId ? undefined : $t('adminSceneSaveFirst')}
      >{sceneMode ? $t('adminSceneBack') : $t('adminSceneMode')}</button>

      <span class="grow"></span>

      <!-- Где байка живёт на сайте. Ссылка — только у той, что на людях:
           у черновика адрес отвечает «не найдено», и ссылка туда была бы
           обещанием, которое страница не сдержит. -->
      {#if siteHref}
        {#if status === 'published'}
          <a href={siteHref} target="_blank" rel="noopener" class="slug-link">{siteHref} ↗</a>
        {:else if slugEditable}
          <label class="slug" title={$t('adminTalesSlugHint')}>
            <span>/tales/</span><input
              bind:value={slugDraft}
              onblur={commitSlug}
              onkeydown={(e) => { if (e.key === 'Enter') e.currentTarget.blur(); else if (e.key === 'Escape') { slugDraft = slug; e.currentTarget.blur(); } }}
              maxlength="120"
              aria-label={$t('adminTalesSlugHint')}
              class="slug-input"
            />
          </label>
        {:else}
          <span class="slug-fixed" title={addressFixed ? $t('adminTalesSlugFixed') : $t('adminTalesSiteHidden')}>{siteHref}</span>
        {/if}
      {/if}
      <button class="tb" onclick={() => (focus = true)}>{$t('adminTalesFocus')}</button>
    </div>

    {#if markupOpen}
      <!-- Что понимает тело байки. Разметки ровно столько, сколько знает
           renderTale; остальное печатается как написано. -->
      <div class="desk-chrome markup">
        <dl>
          <dt># {$t('adminTalesMarkupHeadEx')}</dt><dd>{$t('adminTalesMarkupHead')}</dd>
          <dt>**{$t('adminTalesMarkupBoldEx')}**</dt><dd>{$t('adminTalesMarkupBold')}</dd>
          <dt>&gt; {$t('adminTalesMarkupQuoteEx')}</dt><dd>{$t('adminTalesMarkupQuote')}</dd>
          <dt>{ORNAMENT}</dt><dd>{$t('adminTalesMarkupOrnament')}</dd>
          <dt>ritunia.com/figurines/…</dt><dd>{$t('adminTalesMarkupLink')}</dd>
        </dl>
      </div>
    {/if}


    {#if sceneMode && selectedId}
      <div class="scroll">
        {#key selectedId}
          <TaleSceneEditor taleId={selectedId} {slug} {bodyEn} {bodyRu} lang={editLang} />
        {/key}
      </div>
    {:else}
      <div class="scroll">
        <div class="stage" style="--paper: {both ? 1020 : 780}px">
          <div class="col">
        {#if restorable}
          <div class="restore">
            <span class="grow">{$t('adminTalesRestore')}</span>
            <button onclick={restoreDraft} class="link link--accent">{$t('adminTalesRestoreDo')}</button>
            <button onclick={dropDraft} class="link">{$t('adminTalesRestoreDrop')}</button>
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

          <!-- ── Свойства: работа, обложка, продолжение ─────────────────── -->
          <aside class="inspector desk-chrome" aria-label={$t('adminTalesDetails')}>
            <section class="blk">
              <h4>{$t('adminTalesWork')}</h4>
              <WorkPicker {figurines} bind:value={figurineId} />
            </section>

            <section class="blk">
              <h4>{$t('adminTalesCover')}</h4>
              <div class="cover">
                {#if plate}
                  <img class="cover-img" src={plate} alt="" />
                {:else}
                  <div class="cover-img cover-img--none">{$t('adminTalesCoverEmpty')}</div>
                {/if}
                <div class="cover-acts">
                  <button class="link" onclick={uploadPhoto} disabled={uploading}>
                    {uploading ? '…' : $t('adminTalesUpload')}
                  </button>
                  {#if plate}
                    <button class="link link--danger" onclick={() => (imageUrls = [])}>{$t('adminTalesDropPhoto')}</button>
                  {/if}
                </div>
              </div>

              {#if frames.length > 0}
                <p class="sub-h">{$t('adminTalesWorkPhotos')} · {frames.length}</p>
                <div class="frames">
                  {#each frames as url (url)}
                    <button
                      type="button"
                      onclick={() => pickFrame(url)}
                      title={$t('adminTalesUseWorkPhoto')}
                      class="frame"
                      class:on={plate === url}
                    ><img src={url} alt="" loading="lazy" /></button>
                  {/each}
                </div>
              {/if}
            </section>

            <!-- Продолжение: кто просил продолжение начала, получит письмо, когда
                 эта байка выйдет. Только у сохранённой байки — у несохранённой
                 нет id, на который сослаться. -->
            {#if selectedId}
              <section class="blk">
                <h4>{$t('adminTalesSequelOf')}</h4>
                <select value={mine?.sequelOf ?? ''} onchange={(e) => setSequel(e.currentTarget)} class="select" aria-label={$t('adminTalesSequelOf')}>
                  <option value="">{$t('adminTalesSequelNone')}</option>
                  {#each tales.filter((other) => other.id !== selectedId) as other (other.id)}
                    <option value={other.id}>{titleOf(other)}</option>
                  {/each}
                </select>
              </section>
            {/if}
          </aside>
        </div>
      </div>
    {/if}

    {#if preflight}
      {@const p = preflight}
      <!-- Листок перед выходом. Стоит над лентой, у той самой кнопки, а не
           окном поверх бумаги: байку под ним видно, и замечание можно
           проверить глазами, не закрывая его. -->
      <div class="preflight desk-chrome" role="alertdialog" aria-label={$t('adminTalesPreflight')}>
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
        <div class="pf-acts">
          <button
            onclick={confirmRelease}
            class="btn btn--accent"
          >{p.target === 'scheduled' ? $t('adminTalesPreflightSchedule') : $t('adminTalesPreflightGo')}</button>
          <button onclick={cancelRelease} class="btn">
            {$t('adminTalesPreflightBack')}
          </button>
        </div>
      </div>
    {/if}

    <!-- ── Подвал: статус и выход ─────────────────────────────────────── -->
    <footer class="foot desk-chrome">
      <div class="seg" role="group" aria-label={$t('adminTalesState')}>
        {#each [['draft', 'adminTalesStatusDraft'], ['scheduled', 'adminTalesStatusScheduled'], ['published', 'adminTalesStatusPublished'], ['archived', 'adminTalesStatusArchived']] as [value, label]}
          <button
            class:on={status === value || (value === 'scheduled' && scheduling)}
            onclick={() => setStatus(value as GazetteStatus)}
            disabled={!open}
          >{$t(label as never)}</button>
        {/each}
      </div>

      {#if status === 'scheduled' || scheduling}
        <!-- Дата уходит через тот же листок, что и «Опубликовать»: в
             назначенный час байка разойдётся письмами так же. -->
        <input
          type="datetime-local"
          value={scheduledAt ? localInput(scheduledAt) : ''}
          onchange={(e) => pickSchedule(e.currentTarget.value)}
          class="date"
          aria-label={$t('adminTalesStatusScheduled')}
        />
        {#if scheduling && status !== 'scheduled'}
          <span class="hint">{$t('adminTalesPickDate')}</span>
        {/if}
      {/if}

      <label class="check">
        <input type="checkbox" bind:checked={pinned} />
        {$t('adminTalesPinned')}
      </label>

      <div class="acts">
        {#if sealed}
          <span class="seal">{$t('adminTalesPublishedSeal')}</span>
        {/if}
        {#if selectedId}
          <button onclick={destroy} class="link link--danger">{$t('adminTalesDelete')}</button>
        {/if}
        <button
          onclick={publish}
          disabled={!open || saving || preflightBusy || preflight !== null}
          class="btn btn--solid"
        >{$t('adminTalesPublish')}</button>
      </div>
    </footer>
  {/if}

  {#if message}
    <p class="message">{message}</p>
  {/if}
</div>

<style>

  /* ── Каркас: шапка, инструменты, бумага со свойствами, подвал ───────────
     Кнопки не наследуют размер от родителя: глобальное `button { font-size:
     .9375rem }` перебивает его, и «10 px заглавными» на контейнере давало
     15-пиксельные кнопки с разрядкой, а не строку инструментов. Поэтому
     размер, трекинг и высота названы у самих классов. */
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: #f8f1e7;
    color: #34251c;
  }

  .grow { flex: 1 1 0; min-width: 0; }

  .bar,
  .tools,
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 20px;
    flex-wrap: wrap;
  }
  .bar { padding-top: 12px; padding-bottom: 12px; border-bottom: 1px solid rgba(52, 37, 28, 0.1); }
  .tools { padding-top: 7px; padding-bottom: 7px; border-bottom: 1px solid rgba(52, 37, 28, 0.1); background: rgba(52, 37, 28, 0.02); }
  .foot { gap: 12px; padding-top: 10px; padding-bottom: 10px; border-top: 1px solid rgba(52, 37, 28, 0.12); background: #f4ecdf; }

  .acts { display: flex; align-items: center; gap: 16px; margin-left: auto; }

  .saved { font-size: 11px; color: #8a6a55; white-space: nowrap; }

  .btn,
  .tb {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border: 1px solid rgba(52, 37, 28, 0.2);
    border-radius: 0;
    background: transparent;
    font: 500 11px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.06em;
    color: #5f4636;
    white-space: nowrap;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }
  .btn:hover:not(:disabled),
  .tb:hover:not(:disabled) { border-color: rgba(52, 37, 28, 0.5); color: #34251c; }
  .btn:disabled,
  .tb:disabled { opacity: 0.4; cursor: default; }
  .btn.on,
  .tb.on { border-color: rgba(52, 37, 28, 0.5); background: rgba(52, 37, 28, 0.07); color: #34251c; }
  .btn--accent { border-color: rgba(198, 95, 60, 0.5); color: #c65f3c; }
  .btn--accent:hover:not(:disabled) { border-color: #c65f3c; background: rgba(198, 95, 60, 0.07); color: #c65f3c; }
  .btn--solid { height: 34px; padding: 0 20px; border-color: #c65f3c; background: #c65f3c; color: #fff8f0; letter-spacing: 0.1em; text-transform: uppercase; }
  .btn--solid:hover:not(:disabled) { background: #b3522f; border-color: #b3522f; color: #fff8f0; }

  .tb { height: 28px; padding: 0 10px; border-color: transparent; }
  .tb--icon { width: 28px; padding: 0; font-size: 14px; }

  .sep { width: 1px; height: 18px; margin: 0 4px; background: rgba(52, 37, 28, 0.15); }

  .link {
    padding: 0;
    border: 0;
    background: none;
    font: 500 12px/1.2 Inter, system-ui, sans-serif;
    color: #8a6a55;
    text-decoration: underline;
    text-decoration-color: rgba(138, 106, 85, 0.4);
    text-underline-offset: 3px;
    cursor: pointer;
    white-space: nowrap;
  }
  .link:hover:not(:disabled) { color: #34251c; }
  .link:disabled { opacity: 0.5; cursor: progress; }
  .link--accent { color: #c65f3c; }
  .link--danger:hover:not(:disabled) { color: #a33; }

  /* Переключатель из соседних кнопок: один из, а не россыпь. */
  .seg { display: inline-flex; border: 1px solid rgba(52, 37, 28, 0.2); }
  .seg button {
    height: 28px;
    padding: 0 12px;
    border: 0;
    border-right: 1px solid rgba(52, 37, 28, 0.2);
    border-radius: 0;
    background: transparent;
    font: 500 11px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.06em;
    color: #8a6a55;
    white-space: nowrap;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .seg button:last-child { border-right: 0; }
  .seg button:hover:not(:disabled) { color: #34251c; }
  .seg button.on { background: #34251c; color: #f8f1e7; }
  .seg button:disabled { opacity: 0.45; cursor: default; }

  .slug-link,
  .slug-fixed,
  .slug { font: 400 12px/1 Inter, system-ui, sans-serif; color: #8a6a55; }
  .slug-link { color: #c65f3c; text-decoration: none; }
  .slug-link:hover { text-decoration: underline; }
  .slug { display: inline-flex; align-items: center; }
  .slug-input {
    width: 18ch;
    height: 26px;
    padding: 0 4px;
    border: 0;
    border-bottom: 1px solid rgba(52, 37, 28, 0.25);
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    font: 400 12px Inter, system-ui, sans-serif;
    color: #34251c;
    outline: none;
  }
  .slug-input:focus { border-color: #34251c; box-shadow: none; background: transparent; }

  .date,
  .select {
    height: 30px;
    padding: 0 8px;
    border: 1px solid rgba(52, 37, 28, 0.2);
    border-radius: 0;
    background: rgba(255, 255, 255, 0.5);
    font: 400 12px Inter, system-ui, sans-serif;
    color: #34251c;
  }
  .select { width: 100%; text-overflow: ellipsis; }
  .date:focus,
  .select:focus { border-color: #34251c; box-shadow: none; }

  .hint { font-size: 11px; color: #8a6a55; }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font: 400 12px/1.2 Inter, system-ui, sans-serif;
    color: #5f4636;
    cursor: pointer;
  }
  .check input {
    width: 15px;
    height: 15px;
    margin: 0;
    padding: 0;
    accent-color: #c65f3c;
    cursor: pointer;
  }

  .message {
    margin: 0;
    padding: 8px 20px;
    border-top: 1px solid rgba(52, 37, 28, 0.1);
    font-size: 12px;
    color: #6f3b24;
  }

  .focus-exit {
    position: absolute;
    top: 10px;
    right: 14px;
    z-index: 30;
    height: 28px;
    padding: 0 12px;
    border: 1px solid rgba(52, 37, 28, 0.2);
    border-radius: 0;
    background: rgba(248, 241, 231, 0.92);
    font: 500 11px/1 Inter, system-ui, sans-serif;
    letter-spacing: 0.06em;
    color: #8a6a55;
    cursor: pointer;
    opacity: 0.55;
    transition: opacity 0.15s;
  }
  .focus-exit:hover { opacity: 1; color: #34251c; }

  .restore {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 24px;
    padding: 9px 14px;
    border: 1px solid rgba(198, 95, 60, 0.35);
    background: rgba(198, 95, 60, 0.05);
    font-size: 12px;
  }

  /* ── Бумага и свойства рядом ───────────────────────────────────────── */
  .scroll {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    container-type: size;
  }

  .stage {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 40px;
    align-items: start;
    max-width: calc(var(--paper, 780px) + 340px + 48px);
    margin: 0 auto;
    padding: 32px 24px 48px;
  }
  .col { min-width: 0; }

  .inspector {
    position: sticky;
    top: 24px;
    display: grid;
    gap: 22px;
    max-height: calc(100cqh - 48px);
    overflow-y: auto;
    padding: 16px 16px 18px;
    border: 1px solid #d8c6b1;
    background: rgba(255, 255, 255, 0.4);
  }
  .blk h4 {
    margin: 0 0 8px;
    font: 500 10px/1.2 Inter, system-ui, sans-serif;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .cover { display: flex; gap: 14px; align-items: flex-start; margin-top: 2px; }
  .cover-img {
    flex: none;
    width: 96px;
    aspect-ratio: 4 / 5;
    object-fit: cover;
    border: 1px solid #d8c6b1;
    background: #1a120e;
  }
  .cover-img--none {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 8px;
    border-style: dashed;
    border-color: rgba(52, 37, 28, 0.3);
    background: rgba(52, 37, 28, 0.04);
    font: italic 11px/1.3 Inter, system-ui, sans-serif;
    text-align: center;
    color: #8a6a55;
  }
  .cover-acts { display: grid; gap: 10px; padding-top: 2px; }

  .sub-h { margin: 16px 0 8px; font: 400 11px/1.2 Inter, system-ui, sans-serif; color: #8a6a55; }
  .frames {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(44px, 1fr));
    gap: 6px;
    max-height: 124px;
    overflow-y: auto;
    padding: 2px;
  }
  .frame {
    padding: 0;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    border: 1px solid #d8c6b1;
    border-radius: 0;
    background: #1a120e;
    cursor: pointer;
  }
  .frame:hover { border-color: #8a6a55; }
  .frame.on { border-color: #c65f3c; box-shadow: 0 0 0 1px #c65f3c; }
  .frame img { display: block; width: 100%; height: 100%; object-fit: cover; }

  /* Узкое окно: свойства уходят под бумагу, а не сжимают её. */
  @container (max-width: 1000px) {
    .stage { grid-template-columns: minmax(0, 1fr); gap: 28px; max-width: calc(var(--paper, 780px) + 48px); }
    .inspector { position: static; max-height: none; }
  }

  /* Nothing but paper. */
  .focus-mode :global(.desk-chrome) { display: none; }
  .focus-mode .stage { grid-template-columns: minmax(0, 1fr); max-width: calc(var(--paper, 780px) + 48px); }

  /* The desk is the paper: same faces, same measure, same colour the room
     will read it in. Nothing here is a labelled field. */

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

  .preflight {
    padding: 12px 20px;
    border-top: 1px solid rgba(198, 95, 60, 0.3);
    background: rgba(198, 95, 60, 0.05);
    font-size: 12px;
  }
  .pf-acts { display: flex; align-items: center; gap: 10px; margin-top: 10px; }
  .pf-head { margin: 0 0 6px; font-size: 9px; letter-spacing: 0.16em; text-transform: uppercase; color: #6f3b24; }
  .pf-warn { margin: 0 0 6px; padding-left: 16px; color: #6f3b24; line-height: 1.5; }
  .pf-letters { margin: 0; color: #34251c; line-height: 1.5; max-width: 80ch; }
  .pf-letters b { font-family: 'Cormorant Garamond', Georgia, serif; font-size: 16px; font-weight: 500; }
  .pf-nomail { display: block; margin-top: 4px; color: #8a2a2a; }

  @media (prefers-reduced-motion: reduce) {
    .seal { animation: none; }
    .measure-fill { transition: none; }
  }
</style>
