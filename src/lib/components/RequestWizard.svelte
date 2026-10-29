<script lang="ts">
  /**
   * Заявка на фигурку по шагам: больше нажатий, меньше печати.
   * Один мастер на «Хочу похожую» (окно на странице работы) и на /commission
   * (страница). В конце объясняет, что готовую фигурку предложат личной
   * ссылкой на Etsy (`EtsyWhy`).
   */
  import { onDestroy, onMount, tick } from 'svelte';
  import { api, ApiError, resolveMediaUrl } from '$lib/api';
  import { t, lang, locale, type TranslationKey } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { keepEmail } from '$lib/utils/nameEmail';
  import { isValidEmail } from '$lib/validation';
  import { interact, trackForm } from '$lib/analytics';
  import { loadIndex } from '$lib/site-index';
  import { localizeList } from '$lib/workText';
  import { budgetCurrency, currencySign } from '$lib/currency';
  import { focusTrap } from '$lib/actions/focusTrap';
  import { lockBodyScroll } from '$lib/actions/lockBodyScroll';
  import { portal } from '$lib/actions/portal';
  import {
    CONTACT_CHANNELS, KEEP_CHANGE_KEYS, WHO_KEYS, MOOD_KEYS, PROPS_KEYS, SIZE_KEYS,
    DEADLINE_KINDS, BUDGET_KEYS, MAX_GUEST_PHOTOS,
    type ContactChannel, type RequestWish, type SizeKey, type DeadlineKind, type BudgetKey,
  } from '$lib/requestChoices';
  import EtsyWhy from '$lib/components/EtsyWhy.svelte';
  import type { FigurineListItem } from '$lib/types/api';

  export interface WizardSource {
    id: string;
    name: string;
    displayName: string;
    photo?: string | null;
  }

  let {
    source = undefined,
    mode = 'dialog',
    onClose = () => {},
  }: {
    source?: WizardSource;
    mode?: 'dialog' | 'page';
    onClose?: () => void;
  } = $props();

  const DRAFT_KEY = 'gotiga_request_draft';
  const STORE_KEY = 'gotiga_commissions';
  const PENDING_CLAIM_KEY = 'gotiga_pending_claim';
  const MAX_PHOTO_BYTES = 8 * 1024 * 1024;

  type StepId = 'wish' | 'idea' | 'size' | 'deadline' | 'budget' | 'contact' | 'review';
  type Photo = { url: string; thumbUrl?: string | null };
  type ContactRow = { channel: ContactChannel; value: string };

  // ── Ответы ────────────────────────────────────────────────────────────
  // svelte-ignore state_referenced_locally
  let src = $state<WizardSource | null>(source ?? null);
  // svelte-ignore state_referenced_locally
  let wish = $state<RequestWish | null>(source ? 'similar' : null);
  let keep = $state<string[]>([]);
  let change = $state<string[]>([]);
  let who = $state<string[]>([]);
  let mood = $state<string[]>([]);
  let holds = $state<string[]>([]);
  let note = $state('');
  let photos = $state<Photo[]>([]);
  let sizeKey = $state<SizeKey | null>(null);
  let deadlineKind = $state<DeadlineKind | null>(null);
  let deadline = $state('');
  let budgetKey = $state<BudgetKey | null>(null);
  let contacts = $state<ContactRow[]>([]);
  let name = $state('');
  let ageOk = $state(false);
  let website = $state(''); // приманка для ботов

  // ── Состояние мастера ────────────────────────────────────────────────
  let stepIndex = $state(0);
  let attempted = $state(false); // нажали «Дальше» на этом шаге с неполным ответом
  let uploading = $state(0);
  let photoError = $state('');
  let sending = $state(false);
  let sendError = $state('');
  let contactsServerError = $state('');
  let badChannel = $state<string | null>(null);
  let sent = $state(false);
  let ready = $state(false); // черновик прочитан
  let heading = $state<HTMLElement | null>(null);
  let body = $state<HTMLElement | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  const currency = budgetCurrency();
  const sign = currencySign(currency);

  let steps = $derived<StepId[]>(
    source
      ? ['idea', 'size', 'deadline', 'budget', 'contact', 'review']
      : ['wish', 'idea', 'size', 'deadline', 'budget', 'contact', 'review'],
  );
  let step = $derived<StepId>(steps[Math.min(stepIndex, steps.length - 1)]);
  let isLast = $derived(step === 'review');
  let total = $derived(steps.length);

  const KEY_LABELS: Record<string, TranslationKey> = {
    look: 'wizKeyLook', pose: 'wizKeyPose', clothes: 'wizKeyClothes', colors: 'wizKeyColors',
    props: 'wizKeyProps', size: 'wizKeySize',
    witch: 'wizWhoWitch', granny: 'wizWhoGranny', gnome: 'wizWhoGnome', girl: 'wizWhoGirl',
    old_man: 'wizWhoOldMan', creature: 'wizWhoCreature', other: 'wizOther',
    kind: 'wizMoodKind', sly: 'wizMoodSly', dark: 'wizMoodDark', funny: 'wizMoodFunny', calm: 'wizMoodCalm',
    broom: 'wizPropBroom', cat: 'wizPropCat', pumpkin: 'wizPropPumpkin', book: 'wizPropBook',
    lantern: 'wizPropLantern', herbs: 'wizPropHerbs', bird: 'wizPropBird',
  };
  // «Другое» у предметов и у «кто» — одно слово.
  const label = (k: string): string => $t(KEY_LABELS[k]);

  const STEP_TITLE: Record<StepId, TranslationKey> = {
    wish: 'wizWishTitle', idea: 'wizIdeaTitle', size: 'wizSizeTitle', deadline: 'wizDeadlineTitle',
    budget: 'wizBudgetTitle', contact: 'wizContactTitle', review: 'wizReviewTitle',
  };

  // ── Работы для выбора (только когда основы нет) ──────────────────────
  let works = $state<FigurineListItem[]>([]);
  let worksLoaded = $state(false);
  let worksFailed = $state(false);
  let pickQuery = $state('');
  let shownWorks = $derived(localizeList(works, $lang));
  let foundWorks = $derived.by(() => {
    const q = pickQuery.trim().toLowerCase();
    return shownWorks.filter((w) => !q || w.name.toLowerCase().includes(q));
  });

  async function loadWorks() {
    if (worksLoaded) return;
    try {
      works = (await loadIndex()).works;
      worksLoaded = true;
      worksFailed = false;
    } catch {
      worksFailed = true;
    }
  }

  $effect(() => {
    if (step === 'wish' && wish === 'similar') void loadWorks();
  });

  function pickWork(w: FigurineListItem) {
    const base = works.find((x) => x.id === w.id) ?? w;
    src = {
      id: w.id,
      name: base.name,
      displayName: w.name,
      photo: w.faceImageUrl ?? null,
    };
  }

  // ── Выбор плиток ──────────────────────────────────────────────────────
  function toggle(list: string[], k: string): string[] {
    return list.includes(k) ? list.filter((x) => x !== k) : [...list, k];
  }
  function toggleKeep(k: string) {
    keep = toggle(keep, k);
    change = change.filter((x) => x !== k);
  }
  function toggleChange(k: string) {
    change = toggle(change, k);
    keep = keep.filter((x) => x !== k);
  }

  // ── Фото ──────────────────────────────────────────────────────────────
  async function addFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    input.value = '';
    photoError = '';
    for (const file of files) {
      if (photos.length + uploading >= MAX_GUEST_PHOTOS) {
        photoError = $t('wizPhotoLimit');
        break;
      }
      if (!/^image\/(jpeg|png|webp)$/.test(file.type)) {
        photoError = $t('wizPhotoWrongType');
        continue;
      }
      if (file.size > MAX_PHOTO_BYTES) {
        photoError = $t('wizPhotoTooBig');
        continue;
      }
      uploading += 1;
      try {
        const up = await api.uploadCommissionPhoto(file);
        if (up.url) photos = [...photos, { url: up.url, thumbUrl: up.thumbUrl ?? null }];
      } catch (err) {
        const status = (err as { status?: number })?.status;
        photoError = status === 429 ? $t('wizPhotoRate') : status === 413 ? $t('wizPhotoTooBig') : $t('wizPhotoFailed');
      } finally {
        uploading -= 1;
      }
    }
  }
  function removePhoto(i: number) {
    photos = photos.filter((_, n) => n !== i);
    photoError = '';
  }

  // ── Размер ────────────────────────────────────────────────────────────
  const SIZE_TEXT: Record<string, TranslationKey> = {
    xs: 'commissionSizeXs', s: 'commissionSizeS', m: 'commissionSizeM', l: 'commissionSizeL', xl: 'commissionSizeXl',
  };
  /** Высота силуэта в условных единицах: 28, 54, 100, 180, 250 мм. */
  const SIZE_GLYPH: Record<string, number> = { xs: 14, s: 24, m: 36, l: 50, xl: 64 };
  /** Существующая строка «Миниатюра — до 1″ (28 мм)» делится на слово и высоту. */
  function sizeParts(k: string): { name: string; hint: string } {
    const full = $t(SIZE_TEXT[k]);
    const parts = full.split(/\s*[—:]\s+/);
    return parts.length > 1 ? { name: parts[0], hint: parts.slice(1).join(' — ') } : { name: full, hint: '' };
  }

  // ── Срок ──────────────────────────────────────────────────────────────
  function todayIso(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }
  const today = todayIso();
  let dateOk = $derived(/^\d{4}-\d{2}-\d{2}$/.test(deadline) && deadline >= today);
  function fmtDate(iso: string): string {
    const d = new Date(`${iso}T00:00:00`);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString($locale, { day: 'numeric', month: 'long', year: 'numeric' });
  }

  // ── Бюджет ────────────────────────────────────────────────────────────
  function budgetText(k: BudgetKey): string {
    switch (k) {
      case 'lt100': return `${$t('wizBudgetUpTo')} ${sign}100`;
      case '100_200': return `${sign}100–200`;
      case '200_400': return `${sign}200–400`;
      case 'gt400': return `${$t('wizBudgetOver')} ${sign}400`;
      default: return $t('wizBudgetDiscuss');
    }
  }

  // ── Способы связи ─────────────────────────────────────────────────────
  const CHANNEL_NAME: Record<ContactChannel, string | TranslationKey> = {
    email: 'wizChEmail', telegram: 'Telegram', whatsapp: 'WhatsApp', viber: 'Viber', signal: 'Signal',
    instagram: 'Instagram', facebook: 'Facebook', x: 'X', phone: 'wizChPhone',
  };
  const channelName = (c: ContactChannel): string =>
    c === 'email' || c === 'phone' ? $t(CHANNEL_NAME[c] as TranslationKey) : (CHANNEL_NAME[c] as string);
  const PHONE_LIKE = new Set<ContactChannel>(['phone', 'whatsapp', 'viber', 'signal']);

  function addContact(channel: ContactChannel) {
    if (contacts.some((c) => c.channel === channel)) {
      contacts = contacts.filter((c) => c.channel !== channel);
      return;
    }
    contacts = [...contacts, { channel, value: '' }];
    contactsServerError = '';
    void tick().then(() => document.getElementById(`wiz-contact-${channel}`)?.focus({ preventScroll: false }));
  }
  function removeContact(channel: ContactChannel) {
    contacts = contacts.filter((c) => c.channel !== channel);
  }

  /** Лёгкая проверка, повторяющая правила сервера. '' — всё хорошо. */
  function contactProblem(c: ContactRow): '' | TranslationKey {
    const v = c.value.trim();
    if (!v) return 'wizContactEmpty';
    if (c.channel === 'email') return isValidEmail(v) ? '' : 'wizContactBadEmail';
    if (PHONE_LIKE.has(c.channel)) {
      const digits = v.replace(/\D/g, '');
      return /^[+\d\s().-]+$/.test(v) && digits.length >= 7 && digits.length <= 15 ? '' : 'wizContactBadPhone';
    }
    if (c.channel === 'facebook') return v.length >= 3 && !/\s/.test(v) ? '' : 'wizContactBadHandle';
    const h = v.replace(/^@/, '').replace(/^https?:\/\/[^/]+\//i, '').replace(/\/+$/, '');
    const limits: Record<string, [number, number]> = { telegram: [5, 32], instagram: [1, 30], x: [1, 15] };
    const [min, max] = limits[c.channel] ?? [1, 64];
    if (!/^[A-Za-z0-9_.]+$/.test(h) || h.length < min || h.length > max) {
      return c.channel === 'telegram' ? 'wizContactBadHandleTelegram' : 'wizContactBadHandle';
    }
    return '';
  }

  let contactErrors = $derived(
    contacts.map((c) => ({ channel: c.channel, problem: contactProblem(c) })),
  );
  let contactsOk = $derived(contacts.length > 0 && contactErrors.every((e) => !e.problem));
  let emailValue = $derived(contacts.find((c) => c.channel === 'email')?.value.trim() ?? '');

  // ── Можно ли идти дальше ──────────────────────────────────────────────
  let stepReady = $derived.by(() => {
    switch (step) {
      case 'wish': return wish === 'new' || (wish === 'similar' && src !== null);
      case 'idea': return true;
      case 'size': return sizeKey !== null;
      case 'deadline': return deadlineKind !== null && (deadlineKind !== 'date' || dateOk);
      case 'budget': return budgetKey !== null;
      case 'contact': return contactsOk && name.trim().length > 0 && ageOk;
      default: return true;
    }
  });

  // На странице у формы нет своей прокрутки — к началу шага едет окно, и только если
  // начало формы ушло выше шапки сайта.
  function toStepTop() {
    if (mode !== 'page') { body?.scrollTo({ top: 0 }); return; }
    const top = body?.closest('.wiz')?.getBoundingClientRect().top ?? 0;
    const header = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--site-header-h')) || 0;
    if (top < header) window.scrollBy({ top: top - header - 12 });
  }

  async function goTo(i: number) {
    stepIndex = Math.max(0, Math.min(i, steps.length - 1));
    attempted = false;
    await tick();
    toStepTop();
    heading?.focus({ preventScroll: true });
  }

  function next() {
    if (sending) return;
    if (!stepReady) {
      attempted = true;
      return;
    }
    if (isLast) void submit();
    else void goTo(stepIndex + 1);
  }
  function back() {
    if (stepIndex > 0) void goTo(stepIndex - 1);
  }
  function jump(id: StepId) {
    const i = steps.indexOf(id);
    if (i >= 0) void goTo(i);
  }

  function onKeydown(e: KeyboardEvent) {
    if (mode === 'dialog' && e.key === 'Escape') {
      e.stopPropagation();
      onClose();
      return;
    }
    if (e.key !== 'Enter' || e.shiftKey || e.isComposing) return;
    const el = e.target as HTMLElement;
    // Enter на кнопке нажимает кнопку, в ссылке открывает ссылку.
    if (el.tagName === 'INPUT' && (el as HTMLInputElement).type !== 'checkbox' && (el as HTMLInputElement).type !== 'file' && !isLast) {
      e.preventDefault();
      next();
    }
  }

  // ── Шаги в замер ──────────────────────────────────────────────────────
  const seen = new Set<string>();
  $effect(() => {
    const s = step;
    if (!ready || sent || seen.has(s)) return;
    seen.add(s);
    interact('wizard_step', s, null, false);
  });

  // ── Черновик ──────────────────────────────────────────────────────────
  function readDraft(): Record<string, unknown> | null {
    try {
      const raw = localStorage.getItem(DRAFT_KEY);
      const d = raw ? JSON.parse(raw) : null;
      return d && typeof d === 'object' && d.v === 1 ? d : null;
    } catch {
      return null;
    }
  }
  const strs = (v: unknown, allowed: readonly string[]): string[] =>
    Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string' && allowed.includes(x)) : [];

  function restoreDraft() {
    const d = readDraft();
    if (d) {
      const dsrc = d.src as WizardSource | null | undefined;
      // Черновик про другую работу не подставляется: основа окна главнее.
      const same = !source || (dsrc && dsrc.id === source.id);
      if (same) {
        if (!source && dsrc && typeof dsrc.id === 'string') src = dsrc;
        if (!source && (d.wish === 'similar' || d.wish === 'new')) wish = d.wish;
        keep = strs(d.keep, KEEP_CHANGE_KEYS);
        change = strs(d.change, KEEP_CHANGE_KEYS);
        who = strs(d.who, WHO_KEYS);
        mood = strs(d.mood, MOOD_KEYS);
        holds = strs(d.props, PROPS_KEYS);
        note = typeof d.note === 'string' ? d.note.slice(0, 300) : '';
        if (Array.isArray(d.photos)) {
          photos = (d.photos as Photo[])
            .filter((p) => p && typeof p.url === 'string')
            .slice(0, MAX_GUEST_PHOTOS);
        }
        if (SIZE_KEYS.includes(d.sizeKey as SizeKey)) sizeKey = d.sizeKey as SizeKey;
        if (DEADLINE_KINDS.includes(d.deadlineKind as DeadlineKind)) deadlineKind = d.deadlineKind as DeadlineKind;
        if (typeof d.deadline === 'string') deadline = d.deadline;
        if (BUDGET_KEYS.includes(d.budgetKey as BudgetKey)) budgetKey = d.budgetKey as BudgetKey;
        if (Array.isArray(d.contacts)) {
          contacts = (d.contacts as ContactRow[])
            .filter((c) => c && CONTACT_CHANNELS.includes(c.channel) && typeof c.value === 'string')
            .map((c) => ({ channel: c.channel, value: c.value }));
        }
        if (typeof d.name === 'string') name = d.name;
        if (typeof d.step === 'number' && d.step > 0) {
          // На проверку сразу не уводим: ответы сначала стоит перечитать.
          stepIndex = Math.min(d.step, steps.length - 2);
        }
      }
    }
    // Вошедшему имя и почта известны.
    if (authStore.isLoggedIn) {
      if (!name.trim()) name = authStore.user?.displayName ?? '';
      const mail = authStore.user?.email;
      if (mail && !contacts.some((c) => c.channel === 'email')) {
        contacts = [{ channel: 'email', value: mail }, ...contacts];
      }
    }
  }

  onMount(() => {
    restoreDraft();
    ready = true;
  });

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (!ready || sent) return;
    const draft = {
      v: 1,
      src, wish, keep, change, who, mood, props: holds, note, photos,
      sizeKey, deadlineKind, deadline, budgetKey, contacts, name,
      step: stepIndex,
    };
    pendingDraft = JSON.stringify(draft);
    clearTimeout(saveTimer);
    saveTimer = setTimeout(flushDraft, 250);
    return () => clearTimeout(saveTimer);
  });
  let pendingDraft: string | null = null;
  function flushDraft() {
    if (pendingDraft === null) return;
    try { localStorage.setItem(DRAFT_KEY, pendingDraft); } catch { /* хранилище закрыто */ }
    pendingDraft = null;
  }
  // Закрыли окно сразу после нажатия — последний ответ всё равно сохраняется.
  onDestroy(() => { if (!sent) flushDraft(); });
  function clearDraft() {
    clearTimeout(saveTimer);
    pendingDraft = null;
    try { localStorage.removeItem(DRAFT_KEY); } catch { /* хранилище закрыто */ }
  }

  // ── Сводка ────────────────────────────────────────────────────────────
  let summary = $derived.by(() => {
    const none = $t('wizSumNothing');
    const list = (xs: string[]) => xs.map(label).join(', ');
    const rows: { label: string; value: string; step: StepId }[] = [];
    rows.push({
      label: $t('wizSumWhat'),
      value: wish === 'similar' && src ? $t('wizSimilarTo').replace('{name}', src.displayName) : $t('wizSumNew'),
      step: source ? 'idea' : 'wish',
    });
    if (wish === 'similar') {
      if (keep.length) rows.push({ label: $t('wizSumKeep'), value: list(keep), step: 'idea' });
      if (change.length) rows.push({ label: $t('wizSumChange'), value: list(change), step: 'idea' });
    } else {
      if (who.length) rows.push({ label: $t('wizWhoTitle'), value: list(who), step: 'idea' });
      if (mood.length) rows.push({ label: $t('wizMoodTitle'), value: list(mood), step: 'idea' });
      if (holds.length) rows.push({ label: $t('wizPropsTitle'), value: list(holds), step: 'idea' });
    }
    if (note.trim()) rows.push({ label: $t('wizSumNote'), value: note.trim(), step: 'idea' });
    if (photos.length) rows.push({ label: $t('wizSumPhotos'), value: String(photos.length), step: 'idea' });
    rows.push({
      label: $t('wizSumSize'),
      value: sizeKey === 'unsure' || !sizeKey ? $t('wizSizeUnsure') : (() => { const p = sizeParts(sizeKey); return p.hint ? `${p.name}, ${p.hint}` : p.name; })(),
      step: 'size',
    });
    rows.push({
      label: $t('wizSumDeadline'),
      value:
        deadlineKind === 'date' ? $t('wizSumDeadlineBy').replace('{date}', fmtDate(deadline))
        : deadlineKind === 'asap' ? $t('wizDeadlineAsap')
        : deadlineKind === 'none' ? $t('wizDeadlineNone') : none,
      step: 'deadline',
    });
    rows.push({ label: $t('wizSumBudget'), value: budgetKey ? budgetText(budgetKey) : none, step: 'budget' });
    rows.push({
      label: $t('wizSumContacts'),
      value: contacts.map((c) => `${channelName(c.channel)}: ${c.value.trim()}`).join(' · ') || none,
      step: 'contact',
    });
    rows.push({ label: $t('wizSumName'), value: name.trim() || none, step: 'contact' });
    return rows;
  });

  // ── Отправка ──────────────────────────────────────────────────────────
  function rememberToken(token: string) {
    try {
      const raw = localStorage.getItem(STORE_KEY);
      const list: string[] = raw ? JSON.parse(raw) : [];
      list.push(token);
      localStorage.setItem(STORE_KEY, JSON.stringify(list));
    } catch { /* ignore */ }
  }

  async function submit() {
    sendError = '';
    contactsServerError = '';
    badChannel = null;
    if (!contactsOk || !name.trim() || !ageOk) {
      jump('contact');
      attempted = true;
      return;
    }
    if (authStore.needsEmail && emailValue && isValidEmail(emailValue)) {
      if ((await keepEmail(emailValue)) === 'taken') {
        sendError = $t('formEmailTaken');
        return;
      }
    }
    const similar = wish === 'similar' && src;
    const choices = similar
      ? { keep, change }
      : { who, mood, props: holds };
    sending = true;
    try {
      const res = await api.submitCommission(
        {
          requesterName: name.trim(),
          requesterEmail: emailValue || null,
          contacts: contacts
            .filter((c) => c.value.trim())
            .map((c) => ({ channel: c.channel, value: c.value.trim() })),
          wish: wish ?? 'new',
          choices: choices as never,
          sizeKey,
          deadlineKind,
          deadline: deadlineKind === 'date' ? deadline : null,
          budgetKey,
          budgetCurrency: currency,
          title: similar ? `${$t('commissionInspiredTitlePrefix')} ${src!.name}` : $t('wizTitleNew'),
          description: note.trim(),
          sourceFigurineId: similar ? src!.id : null,
          attachmentUrls: photos.map((p) => ({ url: p.url, thumbUrl: p.thumbUrl ?? undefined })),
          website: website || null,
          lang: $lang,
          ageConfirmed: ageOk,
        },
        authStore.sessionToken ?? undefined,
      );
      rememberToken(res.claimToken);
      if (!authStore.isLoggedIn) {
        try { localStorage.setItem(PENDING_CLAIM_KEY, res.claimToken); } catch { /* ignore */ }
      }
      clearDraft();
      sent = true;
      interact('wizard_sent', wish ?? 'new', null, false);
      await tick();
      heading?.focus({ preventScroll: true });
      toStepTop();
    } catch (err) {
      if (err instanceof ApiError && err.status >= 400 && err.status < 500) {
        const m = /\((email|telegram|whatsapp|viber|signal|instagram|facebook|x|phone)\)/i.exec(err.message);
        const aboutContacts = m !== null || /contact/i.test(err.message);
        if (aboutContacts) {
          badChannel = m ? m[1].toLowerCase() : null;
          contactsServerError = $t('wizErrContacts');
          jump('contact');
        } else {
          sendError = $t('wizErrServer');
        }
      } else {
        sendError = $t('wizErrNetwork');
      }
    } finally {
      sending = false;
    }
  }

  // Сводка о ссылке на Etsy и «Спам» — только если есть почта.
  let hasEmail = $derived(contacts.some((c) => c.channel === 'email' && c.value.trim()));

  const photoSrc = (p: Photo) => resolveMediaUrl(p.thumbUrl ?? p.url) ?? p.url;
</script>

{#snippet tile(text: string, on: boolean, click: () => void, hint?: string)}
  <button type="button" class="tile" class:on aria-pressed={on} onclick={click}>
    <span class="tile-mark" aria-hidden="true">
      <svg viewBox="0 0 12 12" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.2 6.4 4.8 9 9.8 3.2"/></svg>
    </span>
    <span class="tile-text">{text}</span>
    {#if hint}<span class="tile-hint">{hint}</span>{/if}
  </button>
{/snippet}

{#snippet figure(h: number, dashed: boolean)}
  {@const top = 67 - h}
  {@const r = h * 0.13}
  {@const half = h * 0.26}
  <svg class="fig" viewBox="0 0 40 70" width="40" height="70" aria-hidden="true">
    <line x1="4" y1="67" x2="36" y2="67" stroke="currentColor" stroke-width="1" opacity="0.5" />
    {#if dashed}
      <text x="20" y="56" text-anchor="middle" font-size="30" fill="currentColor" font-family="Georgia, serif">?</text>
    {:else}
      <circle cx="20" cy={top + r} r={r} fill="currentColor" />
      <path d="M20 {top + h * 0.28} C{20 - half * 0.5} {67 - h * 0.6}, {20 - half * 0.95} 65, {20 - half} 67 H{20 + half} C{20 + half * 0.95} 65, {20 + half * 0.5} {67 - h * 0.6}, 20 {top + h * 0.28} Z" fill="currentColor" />
    {/if}
  </svg>
{/snippet}

{#snippet glyph(c: ContactChannel)}
  <svg class="glyph" viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#if c === 'email'}
      <rect x="3" y="5.5" width="18" height="13" rx="2" /><path d="m3.5 7 8.5 6.5L20.5 7" />
    {:else if c === 'telegram'}
      <path d="M21 4 3 11l5.5 2L10 19l3-3.5 4.5 3.3L21 4Z" /><path d="m8.5 13 8-5.5" />
    {:else if c === 'whatsapp'}
      <path d="M4 20l1.2-4.2A8 8 0 1 1 8.3 19L4 20Z" /><path d="M9 8.8c.3 2.6 2.700 4.800 5.700 5.700l1.300-1.300-2-1-.9.9c-1-.4-1.900-1.300-2.300-2.300l.9-.9-1-2L9 8.800Z" />
    {:else if c === 'viber'}
      <path d="M5 5c-1 .8-1.500 2-1.300 4 .3 3.500 1.500 6 3.500 8l.3 3.200 2.300-2.200c2.600.2 5.200-.2 7-1.300 1.400-.9 1.800-2.800 1.200-6.200-.4-2.600-1.500-4-3.500-4.500C12.800 5.300 8 4 5 5Z" /><path d="M12 8.500c1.300.2 2.200 1 2.400 2.400M10 9c0 3 2 5 5 5l.8-1-1.700-1-.8.700c-1-.4-1.800-1.200-2.200-2.200l.7-.8-1-1.700-.8 1Z" />
    {:else if c === 'signal'}
      <circle cx="12" cy="12" r="7.500" stroke-dasharray="2.400 2.400" /><path d="M12 6.500a5.500 5.500 0 0 0-4.700 8.300L6.500 17.500l2.700-.8A5.500 5.500 0 1 0 12 6.500Z" />
    {:else if c === 'instagram'}
      <rect x="4" y="4" width="16" height="16" rx="4.500" /><circle cx="12" cy="12" r="3.600" /><circle cx="16.800" cy="7.200" r="0.600" fill="currentColor" />
    {:else if c === 'facebook'}
      <path d="M14 21v-8h2.700l.5-3H14V8.200c0-1 .5-1.700 1.800-1.700H17.300V3.800C16.900 3.700 15.900 3.600 15 3.600c-2.300 0-3.800 1.400-3.800 3.900V10H8.500v3h2.700v8H14Z" />
    {:else if c === 'x'}
      <path d="M4.500 4.500 19.500 19.500M19.500 4.500 4.500 19.500" />
    {:else}
      <path d="M6.500 3.500h3l1.500 4-2 1.300a11 11 0 0 0 5.200 5.200l1.300-2 4 1.500v3a2 2 0 0 1-2 2C10.400 18.500 5.500 13.600 4.500 5.500a2 2 0 0 1 2-2Z" />
    {/if}
  </svg>
{/snippet}

{#snippet pane()}
  <div class="panehead">
    <div class="progress" role="progressbar" aria-valuemin="1" aria-valuemax={total} aria-valuenow={sent ? total : stepIndex + 1} aria-label={$t('wizDialogLabel')}>
      <span class="progress-fill" style="width: {sent ? 100 : ((stepIndex + 1) / total) * 100}%"></span>
    </div>
    <div class="headrow">
      <p class="stepof">{sent ? $t('wizDoneTitle') : $t('wizStepOf').replace('{n}', String(stepIndex + 1)).replace('{m}', String(total))}</p>
      {#if mode === 'dialog'}
        <button type="button" class="close" onclick={onClose} aria-label={$t('lightboxClose')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M1 1l12 12M13 1L1 13" /></svg>
        </button>
      {/if}
    </div>
  </div>

  <div class="panebody" bind:this={body}>
    {#key sent ? 'sent' : step}
      <div class="step">
        {#if sent}
          <h2 class="h" tabindex="-1" bind:this={heading}>{$t('wizDoneTitle')}</h2>
          <ol class="next-list">
            <li>{$t('wizNext1')}</li><li>{$t('wizNext2')}</li><li>{$t('wizNext3')}</li><li>{$t('wizNext4')}</li>
          </ol>
          {#if hasEmail}<p class="quiet">{$t('wizDoneSpam')}</p>{/if}
          <p class="done-actions">
            <a class="btn btn-primary" href="/figurines" onclick={() => { if (mode === 'dialog') onClose(); }}>{$t('wizDoneBack')}</a>
          </p>
        {:else}
          <h2 class="h" tabindex="-1" bind:this={heading}>{$t(STEP_TITLE[step])}</h2>

          {#if step === 'wish'}
            <div class="tiles tiles--two">
              <button type="button" class="tile tile--big" class:on={wish === 'similar'} aria-pressed={wish === 'similar'} onclick={() => (wish = 'similar')}>
                <span class="tile-mark" aria-hidden="true"><svg viewBox="0 0 12 12" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.2 6.4 4.8 9 9.8 3.2"/></svg></span>
                <span class="tile-text">{$t('wizWishSimilar')}</span>
                <span class="tile-hint">{$t('wizWishSimilarHint')}</span>
              </button>
              <button type="button" class="tile tile--big" class:on={wish === 'new'} aria-pressed={wish === 'new'} onclick={() => (wish = 'new')}>
                <span class="tile-mark" aria-hidden="true"><svg viewBox="0 0 12 12" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.2 6.4 4.8 9 9.8 3.2"/></svg></span>
                <span class="tile-text">{$t('wizWishNew')}</span>
                <span class="tile-hint">{$t('wizWishNewHint')}</span>
              </button>
            </div>

            {#if wish === 'similar'}
              <div class="picker">
                <h3 class="sub">{$t('wizPickTitle')}</h3>
                {#if src}
                  <p class="chosen">
                    {#if src.photo}<img src={resolveMediaUrl(src.photo) ?? src.photo} alt="" class="chosen-img" />{/if}
                    <span>{$t('wizPickChosen').replace('{name}', src.displayName)}</span>
                  </p>
                {/if}
                <input
                  class="input"
                  type="search"
                  bind:value={pickQuery}
                  placeholder={$t('wizPickSearch')}
                  aria-label={$t('wizPickSearch')}
                  autocomplete="off"
                />
                {#if worksFailed}
                  <p class="error" role="alert">{$t('wizPickLoadError')}</p>
                {:else if !worksLoaded}
                  <p class="quiet">{$t('wizPhotoUploading')}</p>
                {:else if foundWorks.length === 0}
                  <p class="quiet">{$t('wizPickNone')}</p>
                {:else}
                  <ul class="works">
                    {#each foundWorks as w (w.id)}
                      <li>
                        <button type="button" class="work" class:on={src?.id === w.id} aria-pressed={src?.id === w.id} onclick={() => pickWork(w)}>
                          <span class="work-img">
                            {#if w.faceImageUrl}<img src={resolveMediaUrl(w.faceImageUrl) ?? w.faceImageUrl} alt="" loading="lazy" />{/if}
                          </span>
                          <span class="work-name">{w.name}</span>
                        </button>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          {/if}

          {#if step === 'idea'}
            {#if wish === 'similar' && src}
              <p class="chosen">
                {#if src.photo}<img src={resolveMediaUrl(src.photo) ?? src.photo} alt="" class="chosen-img" />{/if}
                <span>{$t('wizSimilarTo').replace('{name}', src.displayName)}</span>
              </p>
              <p class="quiet">{$t('wizMany')}</p>
              <h3 class="sub">{$t('wizKeepTitle')}</h3>
              <div class="tiles">
                {#each KEEP_CHANGE_KEYS as k}
                  {@render tile(label(k), keep.includes(k), () => toggleKeep(k))}
                {/each}
              </div>
              <h3 class="sub">{$t('wizChangeTitle')}</h3>
              <div class="tiles">
                {#each KEEP_CHANGE_KEYS as k}
                  {@render tile(label(k), change.includes(k), () => toggleChange(k))}
                {/each}
              </div>
            {:else}
              <p class="quiet">{$t('wizMany')}</p>
              <h3 class="sub">{$t('wizWhoTitle')}</h3>
              <div class="tiles">
                {#each WHO_KEYS as k}
                  {@render tile(label(k), who.includes(k), () => (who = toggle(who, k)))}
                {/each}
              </div>
              <h3 class="sub">{$t('wizMoodTitle')}</h3>
              <div class="tiles">
                {#each MOOD_KEYS as k}
                  {@render tile(label(k), mood.includes(k), () => (mood = toggle(mood, k)))}
                {/each}
              </div>
              <h3 class="sub">{$t('wizPropsTitle')}</h3>
              <div class="tiles">
                {#each PROPS_KEYS as k}
                  {@render tile(label(k), holds.includes(k), () => (holds = toggle(holds, k)))}
                {/each}
              </div>
            {/if}

            <label class="field">
              <span class="field-label">{$t('wizNoteLabel')}</span>
              <input class="input" type="text" bind:value={note} maxlength="300" placeholder={$t('wizNotePlaceholder')} autocomplete="off" />
            </label>

            <div class="photos">
              <h3 class="sub">{$t('wizPhotosTitle')}</h3>
              <ul class="thumbs">
                {#each photos as p, i (p.url)}
                  <li class="thumb">
                    <img src={photoSrc(p)} alt="" />
                    <button type="button" class="thumb-x" onclick={() => removePhoto(i)} aria-label={$t('wizPhotoRemove')}>
                      <svg width="12" height="12" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M2 2l10 10M12 2L2 12" /></svg>
                    </button>
                  </li>
                {/each}
                {#each Array(uploading) as _, i (i)}
                  <li class="thumb thumb--busy" aria-live="polite">
                    <span class="bar" aria-hidden="true"></span>
                    <span class="busy-text">{$t('wizPhotoUploading')}</span>
                  </li>
                {/each}
                {#if photos.length + uploading < MAX_GUEST_PHOTOS}
                  <li>
                    <button type="button" class="thumb thumb--add" onclick={() => fileInput?.click()}>
                      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M9 3v12M3 9h12" /></svg>
                      <span>{$t('wizPhotosAdd')}</span>
                    </button>
                  </li>
                {/if}
              </ul>
              <input bind:this={fileInput} class="file" type="file" accept="image/jpeg,image/png,image/webp" multiple onchange={addFiles} tabindex="-1" aria-hidden="true" />
              {#if photoError}<p class="error" role="alert">{photoError}</p>{/if}
            </div>
          {/if}

          {#if step === 'size'}
            <p class="quiet">{$t('wizSizeHint')}</p>
            <div class="tiles tiles--sizes">
              {#each SIZE_KEYS as k}
                {@const on = sizeKey === k}
                <button type="button" class="tile tile--size" class:on aria-pressed={on} onclick={() => (sizeKey = k)}>
                  <span class="tile-mark" aria-hidden="true"><svg viewBox="0 0 12 12" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.2 6.4 4.8 9 9.8 3.2"/></svg></span>
                  {@render figure(k === 'unsure' ? 44 : SIZE_GLYPH[k], k === 'unsure')}
                  {#if k === 'unsure'}
                    <span class="tile-text">{$t('wizSizeUnsure')}</span>
                    <span class="tile-hint">{$t('wizSizeUnsureHint')}</span>
                  {:else}
                    {@const p = sizeParts(k)}
                    <span class="tile-text">{p.name}</span>
                    {#if p.hint}<span class="tile-hint">{p.hint}</span>{/if}
                  {/if}
                </button>
              {/each}
            </div>
          {/if}

          {#if step === 'deadline'}
            <div class="tiles tiles--stack">
              {@render tile($t('wizDeadlineNone'), deadlineKind === 'none', () => (deadlineKind = 'none'))}
              {@render tile($t('wizDeadlineDate'), deadlineKind === 'date', () => (deadlineKind = 'date'))}
              {@render tile($t('wizDeadlineAsap'), deadlineKind === 'asap', () => (deadlineKind = 'asap'))}
            </div>
            {#if deadlineKind === 'date'}
              <label class="field">
                <span class="field-label">{$t('wizDeadlineDateLabel')}</span>
                <input class="input input--date" type="date" min={today} bind:value={deadline} />
              </label>
              {#if attempted && !dateOk}<p class="error" role="alert">{$t('wizDateNeed')}</p>{/if}
            {/if}
          {/if}

          {#if step === 'budget'}
            <div class="tiles tiles--budget">
              {#each BUDGET_KEYS as k}
                {@render tile(budgetText(k), budgetKey === k, () => (budgetKey = k))}
              {/each}
            </div>
            <p class="quiet">{$t('wizBudgetNote')}</p>
          {/if}

          {#if step === 'contact'}
            <p class="quiet">{$t('wizContactHint')}</p>
            <div class="tiles tiles--channels">
              {#each CONTACT_CHANNELS as c}
                {@const on = contacts.some((x) => x.channel === c)}
                <button type="button" class="tile tile--channel" class:on aria-pressed={on} onclick={() => addContact(c)}>
                  <span class="tile-mark" aria-hidden="true"><svg viewBox="0 0 12 12" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2.2 6.4 4.8 9 9.8 3.2"/></svg></span>
                  {@render glyph(c)}
                  <span class="tile-text">{channelName(c)}</span>
                </button>
              {/each}
            </div>

            {#each contacts as c (c.channel)}
              {@const problem = contactProblem(c)}
              <div class="crow">
                <label class="field crow-field" for="wiz-contact-{c.channel}">
                  <span class="field-label">{channelName(c.channel)}</span>
                  {#if c.channel === 'email'}
                    <input id="wiz-contact-{c.channel}" class="input" type="email" autocomplete="email" inputmode="email" placeholder="name@example.com" bind:value={c.value} />
                  {:else if PHONE_LIKE.has(c.channel)}
                    <input id="wiz-contact-{c.channel}" class="input" type="tel" inputmode="tel" autocomplete="tel" placeholder="+373 …" bind:value={c.value} />
                  {:else}
                    <input id="wiz-contact-{c.channel}" class="input" type="text" inputmode="text" autocomplete="off" autocapitalize="none" spellcheck="false" placeholder={$t('wizHandlePlaceholder')} bind:value={c.value} />
                  {/if}
                </label>
                <button type="button" class="crow-x" onclick={() => removeContact(c.channel)} aria-label="{$t('wizContactRemove')}: {channelName(c.channel)}">
                  <svg width="12" height="12" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M2 2l10 10M12 2L2 12" /></svg>
                </button>
                {#if (attempted || badChannel === c.channel || c.value.trim()) && problem && (attempted || c.value.trim())}
                  <p class="error crow-err" role="alert">{$t(problem)}</p>
                {/if}
              </div>
            {/each}
            {#if attempted && contacts.length === 0}<p class="error" role="alert">{$t('wizContactNeed')}</p>{/if}
            {#if contactsServerError}<p class="error" role="alert">{contactsServerError}</p>{/if}

            <label class="field">
              <span class="field-label">{$t('wizNameLabel')}</span>
              <input class="input" type="text" bind:value={name} autocomplete="name" maxlength="80" />
            </label>
            {#if attempted && !name.trim()}<p class="error" role="alert">{$t('wizNameNeed')}</p>{/if}

            <label class="check">
              <input type="checkbox" bind:checked={ageOk} />
              <span>{$t('formAgeConfirm')}</span>
            </label>
            {#if attempted && !ageOk}<p class="error" role="alert">{$t('formAgeConfirmRequired')}</p>{/if}

            <p class="quiet">{$t('wizPrivacy')}</p>
          {/if}

          {#if step === 'review'}
            <dl class="summary">
              {#each summary as row}
                <div class="sum-row">
                  <dt>{row.label}</dt>
                  <dd>{row.value}</dd>
                  <button type="button" class="link" onclick={() => jump(row.step)} aria-label="{$t('wizEdit')}: {row.label}">{$t('wizEdit')}</button>
                </div>
              {/each}
            </dl>

            <h3 class="sub">{$t('wizNextTitle')}</h3>
            <ol class="next-list">
              <li>{$t('wizNext1')}</li><li>{$t('wizNext2')}</li><li>{$t('wizNext3')}</li><li>{$t('wizNext4')}</li>
            </ol>
            <EtsyWhy compact />
          {/if}

          {#if attempted && !stepReady && (step === 'wish' || step === 'size' || step === 'budget' || (step === 'deadline' && deadlineKind === null))}
            <p class="error" role="alert">{$t('wizChooseOne')}</p>
          {/if}
          {#if sendError}<p class="error" role="alert">{sendError}</p>{/if}
        {/if}
      </div>
    {/key}
  </div>

  {#if !sent}
    <div class="panefoot">
      <button type="button" class="btn btn-ghost" onclick={back} disabled={stepIndex === 0 || sending}>{$t('wizBack')}</button>
      <button type="button" class="btn btn-primary" onclick={next} disabled={sending} aria-disabled={!stepReady && !isLast}>
        {isLast ? (sending ? $t('wizSending') : $t('wizSend')) : $t('wizNext')}
      </button>
    </div>
  {/if}
{/snippet}

{#if mode === 'dialog'}
  <div
    class="backdrop"
    role="presentation"
    use:portal
    use:lockBodyScroll
    onmousedown={(e) => { if (e.target === e.currentTarget) onClose(); }}
  >
    <div
      class="wiz wiz--dialog"
      role="dialog"
      aria-modal="true"
      aria-label={$t('wizDialogLabel')}
      tabindex="-1"
      use:focusTrap
      onkeydown={onKeydown}
    >
      <form
        class="inner"
        novalidate
        use:trackForm={{ name: 'request', accepts: ['commission'] }}
        onsubmit={(e) => e.preventDefault()}
      >
        <input class="trap" type="text" name="website" tabindex="-1" autocomplete="off" aria-hidden="true" bind:value={website} />
        {@render pane()}
      </form>
    </div>
  </div>
{:else}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="wiz wiz--page"
    aria-label={$t('wizDialogLabel')}
    novalidate
    use:trackForm={{ name: 'request', accepts: ['commission'] }}
    onkeydown={onKeydown}
    onsubmit={(e) => e.preventDefault()}
  >
    <input class="trap" type="text" name="website" tabindex="-1" autocomplete="off" aria-hidden="true" bind:value={website} />
    {@render pane()}
  </form>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(44, 23, 16, 0.5);
  }
  .wiz {
    --w-ember: var(--color-ember, #b44e24);
    --w-deep: var(--color-ember-deep, #9a4120);
    --w-ink: var(--color-ink-primary, #2c1710);
    --w-soft: var(--color-ink-tertiary, #7a5035);
    --w-line: #d8c6b1;
    --w-paper: #f8f1e7;
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    color: var(--w-ink);
    font-family: Inter, system-ui, sans-serif;
    background: var(--w-paper);
    border: 1px solid var(--w-line);
    border-radius: 10px;
    outline: none;
  }
  .wiz--dialog {
    max-width: 40rem;
    max-height: min(90vh, 52rem);
    box-shadow: 0 24px 60px -18px rgba(44, 23, 16, 0.55);
  }
  .wiz--page {
    max-width: 44rem;
    margin: 0 auto;
  }
  @media (prefers-reduced-motion: no-preference) {
    .wiz--dialog { animation: wiz-in 0.28s ease-out both; }
    .step { animation: wiz-step 0.22s ease-out both; }
    .progress-fill { transition: width 0.25s ease; }
  }
  @keyframes wiz-in { from { opacity: 0; transform: translateY(14px); } }
  @keyframes wiz-step { from { opacity: 0; transform: translateX(8px); } }

  .inner { display: contents; }
  .trap { position: absolute; left: -9999px; width: 1px; height: 1px; opacity: 0; }

  .panehead { padding: 0.9rem 1.2rem 0.4rem; }
  .progress { height: 4px; border-radius: 2px; background: rgba(216, 198, 177, 0.6); overflow: hidden; }
  .progress-fill { display: block; height: 100%; background: var(--w-ember); }
  .headrow { display: flex; align-items: center; justify-content: space-between; min-height: 2.75rem; }
  .stepof { margin: 0; font-size: 0.85rem; color: var(--w-soft); }
  .close {
    display: inline-flex; align-items: center; justify-content: center;
    width: 2.75rem; height: 2.75rem; margin-right: -0.7rem;
    border: 0; background: transparent; color: var(--w-soft); cursor: pointer; border-radius: 6px;
  }

  .panebody { flex: 1 1 auto; overflow-y: auto; padding: 0.2rem 1.2rem 1rem; overscroll-behavior: contain; }
  .h {
    margin: 0.2rem 0 0.8rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.5rem; font-weight: 500; line-height: 1.2;
    outline: none;
  }
  .sub { margin: 1.1rem 0 0.5rem; font-size: 0.95rem; font-weight: 600; }
  .quiet { margin: 0.2rem 0 0.7rem; font-size: 0.88rem; color: var(--w-soft); line-height: 1.45; }
  .error { margin: 0.4rem 0; font-size: 0.88rem; color: #8a2b12; }

  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr)); gap: 0.55rem; }
  .tiles--two { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .tiles--stack { grid-template-columns: 1fr; }
  .tiles--budget { grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr)); }
  .tiles--sizes { grid-template-columns: repeat(auto-fill, minmax(7.6rem, 1fr)); }
  .tiles--channels { grid-template-columns: repeat(auto-fill, minmax(6.8rem, 1fr)); }

  .tile {
    position: relative;
    display: flex; flex-direction: column; align-items: flex-start; justify-content: center; gap: 0.15rem;
    min-height: 3.2rem; padding: 0.6rem 2rem 0.6rem 0.8rem;
    text-align: left; font: inherit; color: var(--w-ink);
    background: #fffdf8; border: 1px solid var(--w-line); border-radius: 8px; cursor: pointer;
  }
  .tile:hover { border-color: var(--w-ember); }
  .tile.on { border: 1px solid var(--w-ember); box-shadow: inset 0 0 0 1px var(--w-ember); background: var(--color-ember-subtle, #fce8df); }
  .tile-text { font-size: 0.95rem; font-weight: 500; line-height: 1.25; }
  .tile-hint { font-size: 0.8rem; color: var(--w-soft); line-height: 1.3; }
  .tile-mark {
    position: absolute; top: 0.5rem; right: 0.5rem;
    display: none; align-items: center; justify-content: center;
    width: 1.1rem; height: 1.1rem; border-radius: 50%;
    color: #fff9f0; background: var(--w-ember);
  }
  .tile.on .tile-mark { display: inline-flex; }
  .tile--big { min-height: 6rem; padding: 0.9rem 2rem 0.9rem 1rem; }
  .tile--big .tile-text { font-family: Georgia, 'Fraunces', serif; font-size: 1.1rem; }
  .tile--size { align-items: center; text-align: center; padding: 0.7rem 0.4rem 0.6rem; min-height: 9.6rem; justify-content: flex-end; }
  .tile--size .tile-mark { top: 0.4rem; right: 0.4rem; }
  .fig { color: var(--w-deep); margin-bottom: 0.25rem; }
  .tile--channel { align-items: center; text-align: center; padding: 0.7rem 0.3rem; gap: 0.3rem; }
  .tile--channel .tile-mark { top: 0.3rem; right: 0.3rem; width: 1rem; height: 1rem; }
  .glyph { color: var(--w-deep); }

  .field { display: block; margin-top: 1rem; }
  .field-label { display: block; margin-bottom: 0.3rem; font-size: 0.88rem; font-weight: 500; }
  .input {
    width: 100%; min-height: 2.75rem; padding: 0 0.8rem; box-sizing: border-box;
    font: inherit; font-size: 1rem; color: var(--w-ink);
    background: #fffdf8; border: 1px solid var(--w-line); border-radius: 8px;
  }
  .input--date { max-width: 14rem; }
  .input::placeholder { color: var(--w-soft); opacity: 0.8; }
  .check { display: flex; align-items: flex-start; gap: 0.6rem; margin: 1rem 0 0.4rem; font-size: 0.92rem; cursor: pointer; min-height: 2.75rem; }
  .check input { width: 1.3rem; height: 1.3rem; margin-top: 0.1rem; accent-color: var(--w-ember); flex: none; }

  .crow { display: grid; grid-template-columns: 1fr auto; gap: 0 0.3rem; align-items: end; }
  .crow-field { margin-top: 0.8rem; }
  .crow-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 2.75rem; height: 2.75rem; border: 0; background: transparent; color: var(--w-soft); cursor: pointer; border-radius: 8px;
  }
  .crow-err { grid-column: 1 / -1; margin: 0.2rem 0 0; }

  .picker { margin-top: 0.6rem; }
  .chosen { display: flex; align-items: center; gap: 0.7rem; margin: 0 0 0.6rem; font-size: 0.95rem; font-weight: 500; }
  .chosen-img { width: 3.2rem; height: 3.2rem; object-fit: cover; border: 1px solid var(--w-line); border-radius: 6px; }
  .works {
    list-style: none; margin: 0.6rem 0 0; padding: 0.1rem;
    display: grid; grid-template-columns: repeat(auto-fill, minmax(5.6rem, 1fr)); gap: 0.5rem;
    max-height: 17rem; overflow-y: auto;
  }
  .work {
    display: flex; flex-direction: column; gap: 0.25rem; width: 100%; padding: 0.3rem;
    font: inherit; font-size: 0.78rem; text-align: left; color: var(--w-ink);
    background: #fffdf8; border: 1px solid var(--w-line); border-radius: 8px; cursor: pointer;
  }
  .work.on { border-color: var(--w-ember); box-shadow: inset 0 0 0 1px var(--w-ember); background: var(--color-ember-subtle, #fce8df); }
  .work-img { display: block; aspect-ratio: 1; overflow: hidden; border-radius: 5px; background: #eadfce; }
  .work-img img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .work-name { line-height: 1.2; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }

  .thumbs { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 0.6rem; }
  .thumb {
    position: relative; display: flex; align-items: center; justify-content: center; flex-direction: column;
    width: 5.5rem; height: 5.5rem; padding: 0; overflow: hidden;
    border: 1px solid var(--w-line); border-radius: 8px; background: #fffdf8;
  }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .thumb--add { gap: 0.2rem; font: inherit; font-size: 0.78rem; color: var(--w-deep); border-style: dashed; cursor: pointer; }
  .thumb--busy { gap: 0.4rem; font-size: 0.75rem; color: var(--w-soft); }
  .bar { width: 60%; height: 4px; border-radius: 2px; background: rgba(216, 198, 177, 0.7); position: relative; overflow: hidden; }
  .bar::after { content: ''; position: absolute; inset: 0 60% 0 0; background: var(--w-ember); }
  @media (prefers-reduced-motion: no-preference) {
    .bar::after { animation: wiz-bar 1.1s ease-in-out infinite alternate; }
  }
  @keyframes wiz-bar { to { inset: 0 0 0 60%; } }
  .thumb-x {
    position: absolute; top: 0.2rem; right: 0.2rem;
    display: inline-flex; align-items: center; justify-content: center;
    width: 1.7rem; height: 1.7rem; border: 0; border-radius: 50%; cursor: pointer;
    color: #fff9f0; background: rgba(44, 23, 16, 0.75);
  }
  .file { position: absolute; width: 1px; height: 1px; opacity: 0; pointer-events: none; }

  .summary { margin: 0; }
  .sum-row {
    display: grid; grid-template-columns: 6.5rem 1fr auto; gap: 0.2rem 0.6rem; align-items: baseline;
    padding: 0.55rem 0; border-bottom: 1px solid rgba(216, 198, 177, 0.7);
  }
  .sum-row dt { font-size: 0.82rem; color: var(--w-soft); }
  .sum-row dd { margin: 0; font-size: 0.95rem; overflow-wrap: anywhere; }
  .link {
    min-height: 2.75rem; padding: 0 0.4rem; border: 0; background: transparent;
    font: inherit; font-size: 0.88rem; color: var(--w-deep); text-decoration: underline; cursor: pointer;
  }
  .next-list { margin: 0.2rem 0 0.9rem; padding-left: 1.5rem; list-style: decimal; display: grid; gap: 0.35rem; font-size: 0.95rem; line-height: 1.45; }

  .done-actions { margin: 1.2rem 0 0.4rem; }

  .panefoot {
    display: flex; justify-content: space-between; gap: 0.7rem;
    padding: 0.8rem 1.2rem calc(0.8rem + env(safe-area-inset-bottom, 0px));
    border-top: 1px solid var(--w-line); background: var(--w-paper); border-radius: 0 0 10px 10px;
  }
  /* На странице форма идёт общей прокруткой: своя прокрутка обрезала заголовок шага
     и давала второй скролл внутри страницы. Кнопки прилипают к низу окна. */
  .wiz--page .panebody { overflow: visible; }
  .wiz--page .panefoot {
    position: sticky; bottom: 0; z-index: 2;
    box-shadow: 0 -10px 18px -14px rgba(44, 23, 16, 0.35);
  }
  /* Нижняя панель разделов есть только до 1024 px; шире переменная остаётся, а панели нет. */
  @media (max-width: 1024px) {
    .wiz--page .panefoot { bottom: var(--section-bar-h, 0px); }
  }
  .btn {
    display: inline-flex; align-items: center; justify-content: center;
    min-height: 2.9rem; padding: 0 1.4rem; font: inherit; font-size: 0.95rem; font-weight: 600;
    border-radius: 8px; cursor: pointer; text-decoration: none; border: 1px solid transparent;
    text-transform: none; letter-spacing: 0.01em; text-align: center;
  }
  .btn-primary { color: #fff9f0; background: var(--w-deep); border-color: var(--w-deep); }
  .btn-primary:hover { background: var(--w-ink); border-color: var(--w-ink); }
  .btn-primary[aria-disabled='true'] { opacity: 0.55; }
  .btn-ghost { color: var(--w-ink); background: transparent; border-color: var(--w-line); }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn:focus-visible, .tile:focus-visible, .work:focus-visible, .close:focus-visible, .link:focus-visible,
  .thumb:focus-visible, .thumb-x:focus-visible, .crow-x:focus-visible, .input:focus-visible {
    outline: 2px solid var(--w-ember); outline-offset: 2px;
  }
  .check input:focus-visible { outline: 2px solid var(--w-ember); outline-offset: 2px; }

  @media (max-width: 640px) {
    .backdrop { padding: 0; align-items: stretch; }
    .wiz--dialog { max-width: none; max-height: none; height: 100dvh; border: 0; border-radius: 0; }
    .panefoot { border-radius: 0; }
    .panehead, .panebody, .panefoot { padding-left: 1rem; padding-right: 1rem; }
    .h { font-size: 1.35rem; }
    .tiles--two { grid-template-columns: 1fr; }
    .sum-row { grid-template-columns: 1fr auto; }
    .sum-row dt { grid-column: 1 / -1; }
    .btn { padding: 0 0.8rem; }
    .btn-ghost { flex: 0 0 auto; }
    .btn-primary { flex: 1; }
  }
</style>
