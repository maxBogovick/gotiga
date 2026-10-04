import { api } from '$lib/api';
import { authStore } from './auth.svelte';
import { savedFigurines } from './saved-figurines.svelte';
import type {
  UserBookingDto,
  UserOrderDto,
  CommissionDto,
  WaitlistEntryDto,
  GazetteWatchDto,
  MessageThreadDto,
  FigurineListItem,
} from '$lib/types/api';

/**
 * Стол посетителя — всё, что дом знает о вошедшем: брони, заказы, прошения,
 * очередь, слежение, переписка и отложенное.
 *
 * Данные лежат здесь, а не внутри страницы профиля, по двум причинам. Первая:
 * профиль стал не одной страницей, а шестью маршрутами, и шесть копий одной
 * загрузки разошлись бы на первой же правке. Вторая: шапка должна показывать
 * точку «есть новое» с любой страницы сайта, а `unreadCount` жил переменной
 * внутри профиля и считался только при его открытии — то есть ответ автора
 * человек не видел, пока сам не зайдёт и не прокликает два уровня.
 *
 * Загрузка идёт **один раз на сессию** и повторяется только по требованию:
 * `load()` помнит токен, под которым набрал данные, и сам сбрасывается, когда
 * токен сменился. Иначе вход под другим именем показал бы чужой стол.
 */

export type FeedTone = 'pending' | 'confirmed' | 'rejected' | 'neutral' | 'attention';
export type FeedKind = 'booking' | 'order' | 'commission' | 'waitlist';

export interface FeedItem {
  kind: FeedKind;
  status: string;
  id: string;
  title: string;
  date: string;
  tone: FeedTone;
  figurineId?: string;
  threadId: string | null;
  unread: number;
}

/**
 * Одна работа со всеми делами, которые у человека на ней есть. Прошение —
 * предложенная работа — стоит собственной карточкой: у него нет `figurineId`,
 * по которому его можно было бы к чему-то приложить.
 */
export interface WorkCard {
  key: string;
  figurineId?: string;
  title: string;
  items: FeedItem[];
  kinds: FeedKind[];
  unread: number;
  tone: FeedTone;
  date: string;
}

/** Самый «живой» тон среди дел карточки становится её тоном. */
const TONE_RANK: FeedTone[] = ['attention', 'pending', 'confirmed', 'rejected', 'neutral'];

const COMMISSION_STAGES = ['new', 'reviewing', 'accepted', 'in_progress', 'completed'] as const;
export { COMMISSION_STAGES };

function bookingTone(s: string): FeedTone {
  return s === 'confirmed' ? 'confirmed' : s === 'rejected' ? 'rejected' : s === 'cancelled' ? 'neutral' : 'pending';
}
function orderTone(s: string): FeedTone {
  return s === 'replied' ? 'confirmed' : s === 'seen' ? 'neutral' : 'pending';
}
function commissionTone(s: string): FeedTone {
  return s === 'completed' ? 'confirmed'
    : s === 'declined' ? 'rejected'
    : (s === 'accepted' || s === 'in_progress') ? 'attention'
    : 'pending';
}

class UserDesk {
  bookings = $state<UserBookingDto[]>([]);
  orders = $state<UserOrderDto[]>([]);
  commissions = $state<CommissionDto[]>([]);
  waitlist = $state<WaitlistEntryDto[]>([]);
  watches = $state<GazetteWatchDto[]>([]);
  threads = $state<MessageThreadDto[]>([]);

  /**
   * Непрочитанное — **сумма писем**, а не число веток с письмами.
   *
   * Сервер отдаёт рядом два числа: у каждой ветки `unread` — сколько в ней
   * непрочитанных ответов, а в `unread` ответа целиком — сколько веток, в
   * которых они есть (`count_unread_threads`, `COUNT(DISTINCT t.id)`).
   * Разные числа под одним словом: при двух ответах в одной ветке и одном в
   * другой рейка показывала «2», а сумма отметок на строках давала «3».
   * Поэтому общее число считается здесь из тех же строк, которые видит
   * человек, и совпадает с ними по устройству, а не по случайности.
   */
  unread = $derived(this.threads.reduce((sum, th) => sum + th.unread, 0));

  /** Каждая работа, на которую смотрит хоть одно дело или отложенное. */
  figurineById = $state<Map<string, FigurineListItem>>(new Map());

  loading = $state(false);
  loaded = $state(false);
  /** Пусто, пока всё в порядке; текст ошибки — когда стол не набрался. */
  error = $state('');
  /** Сессия истекла на загрузке: страница уводит на вход, стор не знает адресов. */
  expired = $state(false);

  #loadedFor: string | null = null;
  #inflight: Promise<void> | null = null;

  // ── Отложенное живёт в своём сторе: он работает и без входа. Стол только
  //    называет его число, чтобы счётчики разделов читались из одного места.
  get wishlistIds(): string[] {
    return savedFigurines.ids;
  }

  /**
   * `referenceId` ветки указывает на бронь, заказ или прошение, которому она
   * принадлежит, — по нему строка дела узнаёт, что на неё ответили.
   */
  threadsByRef = $derived.by(() => {
    const m = new Map<string, { id: string; unread: number }>();
    for (const th of this.threads) {
      if (!th.referenceId) continue;
      const prev = m.get(th.referenceId);
      m.set(th.referenceId, { id: th.id, unread: (prev?.unread ?? 0) + th.unread });
    }
    return m;
  });

  /** Все дела одной лентой, новое сверху. Без слов: ярлыки берёт шаблон. */
  feed = $derived.by<FeedItem[]>(() => {
    const ref = this.threadsByRef;
    const items: FeedItem[] = [];
    for (const b of this.bookings) {
      const t = ref.get(b.id);
      items.push({
        kind: 'booking', status: b.status, id: b.id, title: b.figurineName, date: b.createdAt,
        tone: bookingTone(b.status), figurineId: b.figurineId, threadId: t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    for (const o of this.orders) {
      const t = ref.get(o.id);
      items.push({
        kind: 'order', status: o.status, id: o.id, title: o.figurineName, date: o.createdAt,
        tone: orderTone(o.status), figurineId: o.figurineId, threadId: t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    for (const c of this.commissions) {
      const t = ref.get(c.id);
      items.push({
        kind: 'commission', status: c.status, id: c.id, title: c.title, date: c.createdAt,
        tone: commissionTone(c.status), threadId: c.threadId ?? t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    for (const w of this.waitlist) {
      const t = ref.get(w.id);
      items.push({
        kind: 'waitlist', status: String(w.position), id: w.id, title: w.figurineName, date: w.createdAt,
        tone: 'neutral', figurineId: w.figurineId, threadId: t?.id ?? null, unread: t?.unread ?? 0,
      });
    }
    return items.sort((a, b) => +new Date(b.date) - +new Date(a.date));
  });

  workCards = $derived.by<WorkCard[]>(() => {
    const groups = new Map<string, FeedItem[]>();
    for (const it of this.feed) {
      const key = it.figurineId ?? `${it.kind}:${it.id}`;
      const arr = groups.get(key);
      if (arr) arr.push(it); else groups.set(key, [it]);
    }
    const cards: WorkCard[] = [];
    for (const [key, items] of groups) {
      items.sort((a, b) => +new Date(b.date) - +new Date(a.date));
      cards.push({
        key,
        figurineId: items[0].figurineId,
        title: items[0].title,
        items,
        kinds: [...new Set(items.map((i) => i.kind))],
        unread: items.reduce((s, i) => s + i.unread, 0),
        tone: TONE_RANK.find((tone) => items.some((i) => i.tone === tone)) ?? 'neutral',
        date: items[0].date,
      });
    }
    return cards.sort((a, b) => +new Date(b.date) - +new Date(a.date));
  });

  bookingById = $derived(new Map(this.bookings.map((b) => [b.id, b])));
  orderById = $derived(new Map(this.orders.map((o) => [o.id, o])));
  commissionById = $derived(new Map(this.commissions.map((c) => [c.id, c])));

  /** Числа разделов: их читают и шапка, и рейка профиля. */
  counts = $derived({
    deals: this.workCards.length,
    unread: this.unread,
    threads: this.threads.length,
    wishlist: this.wishlistIds.length,
    watches: this.watches.length,
  });

  cardByKey(key: string): WorkCard | null {
    return this.workCards.find((c) => c.key === key) ?? null;
  }

  /**
   * Одно число, ради которого шапка и заведена. Стоит одного запроса, поэтому
   * зовётся при появлении шапки, а не весь стол.
   */
  async refreshUnread(): Promise<void> {
    const token = authStore.token;
    if (!token) return;
    try {
      const th = await api.getUserThreads(token);
      if (authStore.token !== token) return;
      this.threads = th.threads;
    } catch {
      /* молча: точка над аватаром — не то, ради чего показывают ошибку */
    }
  }

  /**
   * Набрать стол целиком. Повторный вызов при уже набранном столе ничего не
   * делает — кроме `force`, которым пользуются те места, что сами изменили
   * данные (привязали расписку по коду, удалили прошение).
   */
  async load(options: { force?: boolean } = {}): Promise<void> {
    const token = authStore.token;
    if (!token) return;
    if (token !== this.#loadedFor) this.#clear();
    if (this.loaded && !options.force) return;
    if (this.#inflight) return this.#inflight;
    this.#inflight = this.#load(token);
    try {
      await this.#inflight;
    } finally {
      this.#inflight = null;
    }
  }

  async #load(token: string): Promise<void> {
    this.loading = true;
    this.error = '';
    this.expired = false;
    try {
      const [b, o, th, com, wl, gz] = await Promise.all([
        api.userProfileBookings(token),
        api.userProfileOrders(token),
        api.getUserThreads(token),
        api.getUserCommissions(token),
        api.userProfileWaitlist(token).catch(() => [] as WaitlistEntryDto[]),
        api.userGazetteWatches(token).catch(() => [] as GazetteWatchDto[]),
      ]);
      // Вошли под другим именем, пока мы набирали: чужой стол не показываем.
      if (authStore.token !== token) return;
      this.bookings = b;
      this.orders = o;
      this.threads = th.threads;
      this.commissions = com;
      this.waitlist = wl;
      this.watches = gz;

      await savedFigurines.syncWithServer({ importLocal: false });
      await this.#loadFigurines();
      this.loaded = true;
      this.#loadedFor = token;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '';
      if (msg.includes('401')) {
        authStore.clearSession();
        this.#clear();
        this.expired = true;
        return;
      }
      this.error = msg || 'error';
    } finally {
      this.loading = false;
    }
  }

  /**
   * Фотографии и названия всех работ, на которые смотрит хоть что-нибудь.
   * Один запрос на весь каталог: адресных ручек под «эти двенадцать» нет, а
   * двенадцать запросов вместо одного — хуже.
   */
  async #loadFigurines(): Promise<void> {
    const needed = new Set<string>([
      ...this.wishlistIds,
      ...this.commissions.map((c) => c.sourceFigurineId).filter((id): id is string => Boolean(id)),
      ...this.bookings.map((b) => b.figurineId),
      ...this.orders.map((o) => o.figurineId),
      ...this.waitlist.map((w) => w.figurineId),
    ].filter((id): id is string => Boolean(id)));
    if (needed.size === 0) {
      this.figurineById = new Map();
      return;
    }
    const all = await api.getAllFigurines().catch(() => [] as FigurineListItem[]);
    this.figurineById = new Map(all.map((fig) => [fig.id, fig]));
  }

  /** Ветку открыли — её непрочитанное уходит, а с ним и число над аватаром. */
  markThreadRead(id: string): void {
    const prev = this.threads.find((t) => t.id === id);
    if (!prev || prev.unread === 0) return;
    this.threads = this.threads.map((t) => (t.id === id ? { ...t, unread: 0 } : t));
  }

  addThread(thread: MessageThreadDto): void {
    this.threads = [thread, ...this.threads];
  }

  replaceCommission(updated: CommissionDto): void {
    this.commissions = this.commissions.map((c) => (c.id === updated.id ? updated : c));
  }

  dropCommission(id: string): void {
    this.commissions = this.commissions.filter((c) => c.id !== id);
  }

  dropWatch(id: string): void {
    this.watches = this.watches.filter((w) => w.id !== id);
  }

  #clear(): void {
    this.bookings = [];
    this.orders = [];
    this.commissions = [];
    this.waitlist = [];
    this.watches = [];
    this.threads = [];
    this.figurineById = new Map();
    this.loaded = false;
    this.error = '';
    this.#loadedFor = null;
  }

  /** Вышли — стол убирается. Иначе следующий вошедший увидит чужие дела. */
  reset(): void {
    this.#clear();
    this.expired = false;
  }
}

export const userDesk = new UserDesk();
