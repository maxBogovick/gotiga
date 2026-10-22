// Тексты работы на языке читателя.
//
// Работа хранит английские тексты в своих полях, а остальные языки — в
// `translations` (по языку, только заполненные поля). Здесь одно место, которое
// выбирает: язык читателя, затем запасные языки по `FALLBACK` (`firstText`),
// где английский — само поле работы. Страницы пререндерятся по-английски, а
// перевод подставляется в браузере, когда известен язык.

import { firstText, type Lang } from '$lib/i18n';
import type {
	Figurine,
	FigurineImage,
	FigurineListItem,
	FigurineText,
	FigurineTranslations,
	WorkLang,
} from '$lib/types/api';

export type WorkTextField = keyof FigurineText;

const WORK_LANGS: readonly WorkLang[] = ['ru', 'uk', 'es', 'de', 'it'];

/** Поля работы, у которых бывает перевод (порядок — как в форме админки). */
export const WORK_TEXT_FIELDS: readonly WorkTextField[] = [
	'name',
	'shortText',
	'fullDescription',
	'material',
	'technique',
	'dimensions',
	'series',
	'edition',
	'createdPeriod',
	'careInstructions',
	'provenanceNote',
	'authenticityNote',
	'includedItems',
	'secretText',
];

type WithTexts = Partial<Record<WorkTextField, string | null | undefined>> & {
	translations?: FigurineTranslations;
};

/** Один текст по языку: английский — поле самой работы, прочие — из `translations`. */
export function textIn(
	base: string | null | undefined,
	translations: Partial<Record<WorkLang, string | null | undefined>> | null | undefined,
	l: Lang,
): string {
	if (l === 'en' || !translations) return firstText(l, { en: base });
	return firstText(l, { en: base, ...translations });
}

/** Поле работы на языке читателя; ничего нет — пустая строка. */
export function workText(work: WithTexts | null | undefined, field: WorkTextField, l: Lang): string {
	if (!work) return '';
	const tr = work.translations;
	// Копия для показа хранит английское в `translations.en`: её поле уже переведено.
	const base = tr?.en && field in tr.en ? tr.en[field] : work[field];
	if (l === 'en' || !tr) return (base ?? '').trim();
	const texts: Partial<Record<WorkLang, string | null | undefined>> = {};
	for (const k of WORK_LANGS) texts[k] = tr[k]?.[field];
	return textIn(base, texts, l);
}

/** Название работы на языке читателя. */
export function workName(work: WithTexts | null | undefined, l: Lang): string {
	return workText(work, 'name', l);
}

/** Описание фотографии на языке читателя; нет ни одного — `null`. */
export function imageAlt(image: Pick<FigurineImage, 'altText' | 'altTranslations'>, l: Lang): string | null {
	return textIn(image.altText, image.altTranslations, l) || null;
}

/** Название работы, пришедшее рядом с другой записью (`figurineName` +
 *  `figurineNameTranslations`: бронь, лист вестника, карта). */
export function figurineNameIn(
	rec: { figurineName?: string | null; figurineNameTranslations?: Partial<Record<WorkLang, string>> } | null | undefined,
	l: Lang,
): string {
	if (!rec) return '';
	return textIn(rec.figurineName, rec.figurineNameTranslations, l);
}

/** Поля, которые подменяются в работе. Пустой перевод не затирает английское. */
function swapTexts<T extends WithTexts>(work: T, l: Lang): T {
	const out = { ...work } as Record<string, unknown>;
	const en: FigurineText = { ...work.translations?.en };
	for (const f of WORK_TEXT_FIELDS) if (f in work && !(f in en)) en[f] = work[f] ?? null;
	out.translations = { ...work.translations, en };
	for (const f of WORK_TEXT_FIELDS) {
		if (!(f in work)) continue;
		const v = workText(work, f, l);
		const was = work[f];
		out[f] = v || (was == null ? was : '');
	}
	return out as T;
}

/** Работа из списка, где тексты уже на языке читателя. Английский — та же
 *  запись, без копии. Для показа, не для сохранения. */
export function localizeListItem<T extends FigurineListItem>(work: T, l: Lang): T {
	if (!work.translations || (l === 'en' && !work.translations.en)) return work;
	return swapTexts(work, l);
}

/** Список работ на языке читателя; работа без переводов остаётся той же записью. */
export function localizeList<T extends FigurineListItem>(works: T[], l: Lang): T[] {
	return works.map((w) => localizeListItem(w, l));
}

/** Работа целиком на языке читателя: тексты, подписи фото и похожие/связанные
 *  работы. Для показа посетителю; форма админки держит исходную запись. */
export function localizeWork(work: Figurine, l: Lang): Figurine {
	if (l === 'en' && !work.translations?.en) return work;
	const out = work.translations ? swapTexts(work, l) : { ...work };
	out.images = (work.images ?? []).map((img) =>
		img.altTranslations ? { ...img, altText: imageAlt(img, l) } : img,
	);
	out.relatedItems = localizeList(work.relatedItems ?? [], l);
	return out;
}

/** Английское поле работы, даже у копии для показа: разбор по словам данных
 *  (материалы, «см» в размерах) идёт по английскому. */
export function workBase(work: WithTexts, field: WorkTextField): string {
	const en = work.translations?.en;
	return ((en && field in en ? en[field] : work[field]) ?? '').trim();
}

/** Название работы на других языках — то, что кладут рядом с `figurineName`
 *  в сохранённых бронях (`figurineNameTranslations`). */
export function nameTranslations(work: WithTexts): Partial<Record<WorkLang, string>> {
	const out: Partial<Record<WorkLang, string>> = {};
	for (const k of WORK_LANGS) {
		const v = work.translations?.[k]?.name?.trim();
		if (v) out[k] = v;
	}
	return out;
}
