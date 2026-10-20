import { api } from '$lib/api';
import type { TranslationKey } from '$lib/i18n';
import type { ReadinessReport, WorkGap, WorkGaps } from '$lib/types/api';

// Помощник админки: что у работ и байек не доделано.
//
// Ответ один на три места — метку в реестре, список «Готовность» в форме
// работы и вкладку «Что не доделано», — и держит его этот стор: три загрузки
// одного ответа разошлись бы между собой после первого же сохранения.
//
// Проверка идёт по сохранённому: правка в форме видна здесь после «Сохранить».

/** Поиск обновляется фоном после сохранения; столько ждём, прежде чем
 *  спросить снова, иначе только что сохранённая работа висела бы «поиск не
 *  обновлён» до следующей загрузки. */
const SEARCH_SETTLE_MS = 6000;

class AdminReadinessStore {
  report = $state<ReadinessReport | null>(null);
  loading = $state(false);
  #settle: ReturnType<typeof setTimeout> | null = null;

  byWork = $derived(new Map((this.report?.works ?? []).map((w) => [w.id, w])));

  /** Сколько видимых на сайте вещей с пропусками: число на вкладке. Скрытые
   *  работы — черновики, и на число не влияют, хотя в списке есть. */
  count = $derived(
    (this.report?.works.filter((w) => !w.hidden).length ?? 0) + (this.report?.tales.length ?? 0),
  );

  work(id: string | null | undefined): WorkGaps | null {
    return id ? this.byWork.get(id) ?? null : null;
  }

  async load(): Promise<void> {
    this.loading = true;
    try {
      this.report = await api.adminReadiness();
    } catch {
      // Сервер прежней версии без ручки или обрыв: помощник молчит, админка работает.
    } finally {
      this.loading = false;
    }
  }

  /** «Обновить сейчас»: тот же проход, что у кнопки в «Массово», и сразу
   *  новый ответ помощника. */
  async searchNow(): Promise<number> {
    const r = await api.reindexEmbeddings();
    await this.load();
    return r.indexed;
  }

  /** После сохранения: сразу и ещё раз, когда фоновая переиндексация успеет. */
  afterSave(): void {
    void this.load();
    if (this.#settle) clearTimeout(this.#settle);
    this.#settle = setTimeout(() => void this.load(), SEARCH_SETTLE_MS);
  }
}

export const adminReadiness = new AdminReadinessStore();

type Translate = (key: TranslationKey) => string;

/** Слово для пропуска работы — одно на реестр, форму и вкладку. */
export function workGapText(t: Translate, gap: WorkGap, w: WorkGaps): string {
  if (gap === 'similar') return t('adminGap_similar').replace('{n}', String(w.similarCount));
  return t(`adminGap_${gap}` as TranslationKey);
}

/** Как исправить пропуск работы (подсказка по наведению). */
export function workGapHint(t: Translate, gap: WorkGap): string {
  return t(`adminGapHint_${gap}` as TranslationKey);
}

/** На какой вкладке формы работы это исправляют. Поиск чинит сервер. */
export const WORK_GAP_TAB: Record<WorkGap, 'media' | 'object' | null> = {
  photos: 'media',
  cover: 'media',
  angle: 'media',
  similar: 'object',
  search: null,
};

/** Слово для пропуска байки: `body_ru` → «Нет текста RU». */
export function taleGapText(t: Translate, gap: string): string {
  const [kind, lang] = gap.split('_');
  if (kind === 'photo') return t('adminGapTale_photo');
  const key = kind === 'title' ? 'adminGapTale_title' : 'adminGapTale_body';
  return t(key).replace('{lang}', (lang ?? '').toUpperCase());
}
