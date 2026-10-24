import { browser } from '$app/environment';
import { api } from '$lib/api';
import type { SectionCount } from '$lib/types/api';
import { navPhoto } from '$lib/stores/nav-photo.svelte';

// Числа у пунктов шапки: сколько работ, листов вестника и небылиц, и сколько из
// них появилось с тех пор, как посетитель в последний раз открывал этот раздел.
//
// Память — одна запись в localStorage (`gotiga_nav_seen`): для каждого раздела
// дата самой новой записи, которую человек уже видел. Это та же анонимность,
// что у `gotiga_viewed` и `gotiga_ledger`: ничего не уходит на сервер, кроме
// самой даты в запросе чисел. Дата берётся от сервера, а не от часов браузера,
// поэтому разница часов ничего не сдвигает.
//
// Первый заход новых не показывает: сравнивать не с чем. Метка ставится сразу.

export type NavSection = 'works' | 'gazette' | 'tales';

/** Адрес списка раздела: только на нём раздел считается «просмотренным». */
export const SECTION_HREF: Record<NavSection, string> = {
  works: '/figurines',
  gazette: '/gazette',
  tales: '/tales',
};

const KEY = 'gotiga_nav_seen';
/** Через сколько перечитывать числа в долгой вкладке. */
const KEEP_MS = 5 * 60 * 1000;

type Seen = Partial<Record<NavSection, string>>;
type Counts = Partial<Record<NavSection, SectionCount>>;

function readSeen(): Seen {
  if (!browser) return {};
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return {};
    const out: Seen = {};
    for (const s of Object.keys(SECTION_HREF) as NavSection[]) {
      const v = (parsed as Record<string, unknown>)[s];
      if (typeof v === 'string') out[s] = v;
    }
    return out;
  } catch {
    return {};
  }
}

function writeSeen(seen: Seen) {
  try {
    localStorage.setItem(KEY, JSON.stringify(seen));
  } catch {
    // хранилище закрыто — числа остаются без отметок «новое»
  }
}

class NavCountsStore {
  counts = $state<Counts>({});
  /** Этюдов на людях (число у пункта «Игра»); `null`, пока не пришло. */
  etudes = $state<number | null>(null);

  #seen: Seen = {};
  #readAt = 0;
  #pending: Promise<void> | null = null;

  /** Число на полке; `null`, пока оно не пришло: пустое место лучше нуля. */
  total(section: NavSection): number | null {
    return this.counts[section]?.total ?? null;
  }

  /** Сколько нового с прошлого визита; 0 — ничего не печатать. */
  fresh(section: NavSection): number {
    return this.counts[section]?.fresh ?? 0;
  }

  /** Спрашивает числа, не чаще раза в `KEEP_MS`. Без браузера ничего не делает. */
  load(): Promise<void> {
    if (!browser) return Promise.resolve();
    if (this.#pending && Date.now() - this.#readAt < KEEP_MS) return this.#pending;
    this.#readAt = Date.now();
    this.#seen = readSeen();
    this.#pending = api
      .getSectionCounts(this.#seen)
      .then((res) => {
        const next: Counts = {};
        let changed = false;
        for (const s of Object.keys(SECTION_HREF) as NavSection[]) {
          const c = res[s];
          next[s] = c;
          // Нет метки — это первый заход в раздел: запоминаем, до чего дошли.
          if (!this.#seen[s] && c.latest) {
            this.#seen[s] = c.latest;
            changed = true;
          }
        }
        if (changed) writeSeen(this.#seen);
        this.counts = next;
        this.etudes = typeof res.etudes === 'number' ? res.etudes : null;
        navPhoto.apply(res);
      })
      .catch(() => {
        // Числа — украшение: упали — пункты остаются без них, а в следующий раз спросим снова.
        this.#pending = null;
      });
    return this.#pending;
  }

  /** Человек открыл список раздела: всё, что на нём есть, увидено. */
  markSeen(section: NavSection) {
    const c = this.counts[section];
    if (!c?.latest) return;
    if (this.#seen[section] !== c.latest) {
      this.#seen[section] = c.latest;
      writeSeen(this.#seen);
    }
    if (c.fresh !== 0) this.counts = { ...this.counts, [section]: { ...c, fresh: 0 } };
  }
}

export const navCounts = new NavCountsStore();
