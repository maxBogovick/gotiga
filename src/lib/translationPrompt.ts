/**
 * Массовый перевод работ: задание для модели и разбор её ответа.
 *
 * Вкладка «Переводы» кладёт в буфер обмена задание и список работ, которым не
 * хватает текстов (`getMissingTranslations`). Модель отвечает массивом записей
 * «работа + язык + поля»; ответ вставляется назад и уходит в
 * `importTranslations`. Текст задания лежит только здесь.
 */
import type { MissingTranslations, SectionMissing, TranslationImportItem } from '$lib/types/api';

/** Языки переводов работ: код и название для подписей. */
export const TRANSLATION_LANGS = [
  { code: 'es', name: 'Español' },
  { code: 'de', name: 'Deutsch' },
  { code: 'it', name: 'Italiano' },
  { code: 'ru', name: 'Русский' },
  { code: 'uk', name: 'Українська' },
] as const;

const LIMITS = `name 200, shortText 2000, fullDescription 20000, material 500, technique 500,
dimensions 200, series 200, edition 500, createdPeriod 500, careInstructions 5000,
provenanceNote 5000, authenticityNote 5000, includedItems 5000, secretText 5000, alts — 1000 на подпись`;

/** Задание целиком: правила и данные. */
export function translationPrompt(batch: MissingTranslations): string {
  const data = JSON.stringify(batch.items, null, 1);
  return `Переведи тексты работ сайта Gotiga (авторские готические миниатюры) с английского оригинала на недостающие языки.

Данные ниже — массив работ. У каждой работы:
- "id" — идентификатор работы, его надо вернуть как есть;
- "source" — английский текст, по полям (name, shortText, fullDescription и т. д.);
- "need" — для каждого языка список полей, которые надо перевести. Переводи ТОЛЬКО перечисленные поля и только на этот язык;
- "images" — подписи к фото: "id" фото, "altText" английская подпись, "langs" языки, на которые её надо перевести.

Ответ — один JSON-массив, без пояснений до и после, в одном блоке кода. Одна запись на каждую пару «работа + язык»:
[
  { "id": "<id работы>", "lang": "es", "name": "…", "shortText": "…", "alts": { "<id фото>": "…" } }
]
Поля записи — те же, что в "need" для этого языка. "alts" — подписи фото на этом языке (только для фото, у которых язык есть в "langs"); если подписей нет, поле не пишется. Если для работы и языка нечего переводить, записи нет.

Правила:
- Язык кода: es — испанский, de — немецкий, it — итальянский, ru — русский, uk — украинский. Переводить надо с английского, но если в проекте есть глоссарии (tools/i18n/glossary-{es,de,it}.md), термины берутся из них.
- Названия — прямыми словами, без образов и украшений: переводи то, что сказано, ничего не добавляй и не пропускай. Не сокращай и не пересказывай.
- Абзацы, переносы строк, списки, разметка, числа, размеры и ссылки сохраняются. Единицы измерения (cm, mm, g) пиши так, как принято на языке перевода, числа не меняй.
- Имена собственные и названия серий переводятся только там, где у них есть устоявшаяся форма. Название работы переводи по смыслу, коротко, как название, а не как предложение.
- Кавычки: «…» для es, it, ru, uk; „…“ для de.
- Не выдумывай то, чего нет в оригинале. Если в оригинале явная ошибка, переведи как написано и отметь это одной строкой после блока кода.
- Пределы длины (в знаках): ${LIMITS}.

Работ в задании: ${batch.items.length} из ${batch.total}.

${data}
`;
}

export interface ParsedTranslations {
  items: TranslationImportItem[];
  /** Если разобрать не удалось, причина. */
  error: string | null;
}

/** Достаёт массив записей из ответа модели: с кодовым блоком или без, массивом или `{items}`. */
export function parseTranslations(raw: string): ParsedTranslations {
  let text = raw.trim();
  if (!text) return { items: [], error: null };
  const fence = text.match(/```(?:json)?\s*([\s\S]*?)```/i);
  if (fence) text = fence[1].trim();
  else {
    const open = text.indexOf('[');
    const close = text.lastIndexOf(']');
    if (open >= 0 && close > open) text = text.slice(open, close + 1);
  }
  try {
    const data = JSON.parse(text);
    const list = Array.isArray(data) ? data : Array.isArray(data?.items) ? data.items : null;
    if (!list) return { items: [], error: 'not an array' };
    const items: TranslationImportItem[] = [];
    for (const r of list) {
      if (r && typeof r === 'object' && typeof r.id === 'string' && typeof r.lang === 'string') {
        items.push(r as TranslationImportItem);
      }
    }
    if (items.length !== list.length) {
      return { items, error: `${list.length - items.length} record(s) without id or lang` };
    }
    return { items, error: null };
  } catch (e) {
    return { items: [], error: String(e instanceof Error ? e.message : e) };
  }
}

/** Разделы вкладки «Переводы». `works` — работы, остальные идут через `importSection`. */
export const TRANSLATION_SECTIONS = ['works', 'tales', 'gazette', 'cards', 'site', 'errands', 'terms'] as const;
export type TranslationSection = (typeof TRANSLATION_SECTIONS)[number];

/** Все шесть языков сайта: у не-работ переводится и на английский, если там пусто. */
export const SECTION_LANGS = [
  { code: 'en', name: 'English' },
  ...TRANSLATION_LANGS,
] as const;

const SECTION_RULES: Record<Exclude<TranslationSection, 'works'>, string> = {
  tales: `Это байки (короткие рассказы). Сохраняй абзацы, пустые строки и разметку байки: «# » в начале заголовка, «> » у цитаты, **жирный**, ссылки. Тон и ритм переводи, а не пересказывай. Оригинал почти везде русский (в "source" он под "ru"), у одной байки английский; сверяй с вторым языком, если он есть.`,
  gazette: `Это листы газеты сайта (заголовок, подзаголовок, текст). Сохраняй абзацы и разметку. Новости и анонсы переводи ясно и коротко, без украшений.`,
  cards: `Это карты карточной игры: название, тип, эффект, история; черты ("trait": name, text) и умения ("ability": name). Названия терминов боя бери только из словаря: Защита от чар (Spell defence), Иммунитет (Immune), Подчинение (Dominated), Невидимость (Hidden), Заслон, Передышка (Steadfast), Вид урона (Damage type), Ранг, противник, карта. На es/de/it — по глоссариям tools/i18n/glossary-{es,de,it}.md. Эффект пиши так, чтобы правила не менялись: числа, условия и порядок действий сохраняются.`,
  site: `Это правки текстов сайта: ключ записи — ключ словаря интерфейса (в "name"). Плейсхолдеры в фигурных скобках ({brand}, {n}, {name} и подобные), HTML-теги и ссылки переносятся без изменений. Переводи так, как текст стоит на странице: заголовок — как заголовок, кнопку — как кнопку.`,
  errands: `Это поручения и этюды карточной игры (название и описание для игрока). Описывай то, что нужно сделать, прямыми словами, без образов. Термины боя бери из словаря: Защита от чар, Иммунитет, Подчинение, Невидимость, Заслон, Передышка, ранг, противник, карта; на es/de/it — по глоссариям tools/i18n/glossary-{es,de,it}.md.`,
  terms: `Это термины боя: расы, ключевые слова и их правила, имена рангов рамок, имена движений. Термины должны совпадать с остальным интерфейсом: бери слова из словаря (Защита от чар, Иммунитет, Подчинение, Невидимость, Заслон, Передышка, Вид урона) и из глоссариев tools/i18n/glossary-{es,de,it}.md. Название — коротко, одним-двумя словами.`,
};

const LANG_NAMES: Record<string, string> = {
  en: 'английский',
  ru: 'русский',
  uk: 'украинский',
  es: 'испанский',
  de: 'немецкий',
  it: 'итальянский',
};

/** Задание для модели по записям одного раздела (не работ). */
export function sectionPrompt(section: Exclude<TranslationSection, 'works'>, batch: SectionMissing): string {
  const data = JSON.stringify(batch.items, null, 1);
  const langs = [...new Set(batch.items.flatMap((i) => Object.keys(i.need)))];
  return `Переведи тексты сайта Gotiga (авторские готические миниатюры и карточная игра) на недостающие языки.

Данные ниже — массив записей. У каждой записи:
- "id" — идентификатор, его надо вернуть как есть;
- "name" — подпись для ориентира (переводить её не нужно);
- "source" — существующие тексты по полям и языкам: {"поле": {"ru": "…", "en": "…"}}. Переводи с русского со сверкой по английскому, если оба есть; иначе с того, что есть;
- "need" — для каждого языка список полей, которые надо перевести. Переводи ТОЛЬКО перечисленные поля и только на этот язык;
- "limits" — предел длины каждого поля в знаках.

Ответ — один JSON-массив, без пояснений до и после, в одном блоке кода. Одна запись на каждую пару «запись + язык»:
[
  { "id": "<id записи>", "lang": "${langs[0] ?? 'es'}", "<поле>": "…" }
]
Поля записи — те же, что в "need" для этого языка. Если для записи и языка нечего переводить, записи нет.

Языки: ${langs.map((l) => `${l} — ${LANG_NAMES[l] ?? l}`).join(', ')}.

Правила:
- Названия и тексты — прямыми словами, без образов и украшений: переводи то, что сказано, ничего не добавляй и не пропускай. Не сокращай и не пересказывай.
- Числа, имена собственные, плейсхолдеры и ссылки сохраняются.
- Кавычки: «…» для es, it, ru, uk; „…“ для de.
- Не превышай "limits". Если перевод длиннее, перефразируй короче, не обрезай слово.
- ${SECTION_RULES[section]}

Записей в задании: ${batch.items.length} из ${batch.total}.

${data}
`;
}
