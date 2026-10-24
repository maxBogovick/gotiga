import type { NavSealTale, SectionCounts } from '$lib/types/api';

// Картинки на печатях шапки.
//
// «Все работы» — фото случайной работы: выбор один раз за загрузку страницы и
// держится, пока человек ходит по сайту, — печать, меняющая фото при каждом
// переходе, отвлекала бы от страницы.
//
// «Небылицы» — обложка последней вышедшей небылицы.
//
// Обе приходят в том же ответе, что и числа у пунктов (`GET /site/counts`,
// `navCounts.load`), и сюда их передаёт `apply`. Прежде печати сами тянули полный
// список работ и четыре списка игры (≈340 КБ JSON) ради двух картинок.
// «Игра», «Вестник» и «Студия» — готовые картинки (`static/seals/`), их не
// спрашивают.
//
// Пока картинки нет (или сервер старый и её не прислал), на печати стоит знак.

export interface NavPhoto {
  src: string;
  thumbUrl: string | null;
}

class NavPhotoStore {
  works = $state<NavPhoto | null>(null);
  tale = $state<NavSealTale | null>(null);

  apply(res: Pick<SectionCounts, 'sealWork' | 'sealTale'>): void {
    // Фото работы выбрано один раз: следующие ответы (вкладка долго открыта)
    // печать не меняют.
    if (!this.works && res.sealWork?.src) {
      this.works = { src: res.sealWork.src, thumbUrl: null };
    }
    if (res.sealTale) this.tale = res.sealTale;
  }
}

export const navPhoto = new NavPhotoStore();
