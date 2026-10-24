import { browser } from '$app/environment';
import { api, resolveMediaUrl } from '$lib/api';
import type { NavSealImage, NavSealKey, NavSealSection, NavSeals } from '$lib/types/api';

// Картинки на печатях меню, которые автор загружает в админке («Автор → Печати
// меню»). У раздела одна картинка стоит всегда, несколько — сменяются каждые
// `intervalSec` секунд. Раздел без картинок рисует то, что сайт рисует сам.
//
// Смена идёт отсюда, а не из печати: печать только показывает картинку, которую
// ей дали, и плавно переходит к следующей. Следующая загружается заранее —
// переключение на ещё не пришедшую картинку показало бы пустую печать. Вкладка в
// фоне не меняет картинок, а при `prefers-reduced-motion` они не меняются вовсе.

const KEYS: NavSealKey[] = ['works', 'gazette', 'tales', 'game', 'studio'];
/** Печати меняют картинку не все разом: сдвиг по порядку в ряду. */
const STAGGER_MS = 1400;
/** Для `sizes`: печать с выступами рамки около 64 px. */
export const SEAL_SIZES = '64px';

class NavSealsStore {
  /** Ответ сервера пришёл (пусть и пустой): до этого картинок по умолчанию не показываем. */
  ready = $state(false);
  config = $state<NavSeals>({ sections: {} });
  index = $state<Partial<Record<NavSealKey, number>>>({});
  #started = false;
  #timers: ReturnType<typeof setInterval>[] = [];

  /** Раздел, у которого есть картинки; иначе `null`. */
  of(key: NavSealKey): NavSealSection | null {
    const sec = this.config.sections[key];
    return sec && sec.images.length > 0 ? sec : null;
  }

  /** Картинка, которая стоит на печати сейчас. */
  current(key: NavSealKey): NavSealImage | null {
    const sec = this.of(key);
    if (!sec) return null;
    return sec.images[(this.index[key] ?? 0) % sec.images.length] ?? null;
  }

  /** Читает настройку один раз за загрузку страницы; `force` — прочитать заново. */
  load(force = false): void {
    if (!browser || (this.#started && !force)) return;
    this.#started = true;
    api
      .getNavSeals()
      .then((res) => {
        this.config = res && res.sections ? res : { sections: {} };
        this.#schedule();
      })
      .catch(() => {
        // Старый сервер или обрыв сети: остаются картинки по умолчанию.
        this.config = { sections: {} };
      })
      .finally(() => {
        this.ready = true;
      });
  }

  #schedule() {
    this.#timers.forEach(clearInterval);
    this.#timers = [];
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    KEYS.forEach((key, order) => {
      const sec = this.of(key);
      if (!sec || sec.images.length < 2) return;
      const every = Math.max(5, sec.intervalSec) * 1000;
      setTimeout(() => {
        this.#timers.push(setInterval(() => this.#advance(key), every));
      }, order * STAGGER_MS);
    });
  }

  #advance(key: NavSealKey) {
    if (document.hidden) return;
    const sec = this.of(key);
    if (!sec) return;
    const at = (this.index[key] ?? 0) % sec.images.length;
    const next = (at + 1) % sec.images.length;
    const img = new Image();
    img.sizes = SEAL_SIZES;
    img.srcset = sealSrcset(sec.images[next]);
    img.src = resolveMediaUrl(sec.images[next].url) ?? sec.images[next].url;
    img
      .decode()
      .then(() => {
        this.index = { ...this.index, [key]: next };
      })
      .catch(() => {
        // Картинка не загрузилась: остаёмся на нынешней, в следующий раз попробуем снова.
      });
  }
}

/** `srcset` картинки печати: малая копия для обычного экрана, крупная для остальных. */
export function sealSrcset(img: NavSealImage): string {
  const small = resolveMediaUrl(img.urlSmall) ?? img.urlSmall;
  const large = resolveMediaUrl(img.url) ?? img.url;
  return `${small} ${img.widthSmall}w, ${large} ${img.width}w`;
}

export const navSeals = new NavSealsStore();
