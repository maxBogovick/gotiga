// Команды в окне «Спросить сайт»: «открой игру», «покажи ведьм в архиве»,
// «открой последнюю небылицу», «переключи на английский».
//
// Команда — это вопрос, ответом на который служит действие: окно называет, что
// сделает, и через мгновение делает (переход или смена языка). Пауза нужна,
// чтобы человек успел прочесть и передумать: новый вопрос или закрытое окно
// отменяют действие. Ссылка под ответом остаётся — по ней можно перейти самому.
//
// Только разбор и сборка ответа; само действие исполняет стор.
import { normalizeQuery } from '$lib/keeper-search';
import { leafCopy, leafHref } from '$lib/gazette';
import { figurineHref } from '$lib/figurineHref';
import type { AskIndex } from '$lib/site-index';
import {
  say, quote, workName, leafTitle, newest, talesNewest, FILLER, tokensOf, startsWithAny,
  type AskAction, type AskReply, type Lang,
} from '$lib/site-ask-kit';

export type GoSpec =
  | { kind: 'page'; href: string; label: [string, string, string] }
  | { kind: 'archive'; topic: string }
  | { kind: 'latest'; what: 'work' | 'tale' }
  | { kind: 'random'; what: 'work' | 'tale' }
  | { kind: 'lang'; lang: Lang };

const VERBS = [
  'открой', 'открыть', 'перейди', 'перейти', 'отведи', 'веди ', 'зайди', 'покажи страницу', 'покажи раздел',
  'go to', 'open ', 'take me', 'show me the',
  'відкрий', 'відкрити', 'відведи', 'зайди',
];

const SWITCH_VERBS = ['переключи', 'смени', 'поменяй', 'переведи', 'включи', 'switch', 'change', 'перемкни', 'змін', 'увімкни'];
const LANG_STEMS: [Lang, string[]][] = [
  ['en', ['англ', 'english']],
  ['uk', ['украин', 'укр', 'ukrain', 'українськ']],
  ['ru', ['русск', 'russian', 'російськ', 'рос']],
];

const PAGES: { stems: string[]; href: string; label: [string, string, string] }[] = [
  { stems: ['полку карт', 'полка карт', 'полку', 'shelf', 'полиц'], href: '/battles', label: ['Полка карт', 'Полиця карт', 'Card shelf'] },
  { stems: ['игр', 'этюд', 'бой', 'битв', 'game', 'stud', 'battle', 'гру', 'гра', 'етюд', 'бій'], href: '/battles/etude', label: ['Игра', 'Гра', 'The game'] },
  { stems: ['карт', 'card'], href: '/battles', label: ['Полка карт', 'Полиця карт', 'Card shelf'] },
  { stems: ['небыл', 'байк', 'tales', 'небил'], href: '/tales', label: ['Небылицы', 'Небилиці', 'Tall tales'] },
  { stems: ['вестник', 'газет', 'gazette', 'вісник'], href: '/gazette', label: ['Вестник', 'Вісник', 'Gazette'] },
  { stems: ['мастерск', 'workshop', 'майстерн'], href: '/workshop', label: ['Мастерская', 'Майстерня', 'The workshop'] },
  { stems: ['студи', 'studio'], href: '/studio', label: ['Студия', 'Студія', 'Studio'] },
  { stems: ['об авторе', 'автор', 'author', 'про автора'], href: '/author', label: ['Об авторе', 'Про автора', 'About the author'] },
  { stems: ['приобрест', 'acquire', 'придбат'], href: '/acquire', label: ['Как приобрести работу', 'Як придбати роботу', 'How to acquire a work'] },
  { stems: ['профил', 'profile', 'профіл', 'кабинет'], href: '/profile', label: ['Профиль', 'Профіль', 'Profile'] },
  { stems: ['политик', 'privacy', 'конфиденц', 'політик'], href: '/privacy', label: ['Политика конфиденциальности', 'Політика конфіденційності', 'Privacy policy'] },
  { stems: ['главн', 'домой', 'home', 'головн'], href: '/', label: ['Главная', 'Головна', 'Home'] },
  { stems: ['архив', 'работы', 'все работы', 'каталог', 'works', 'archive', 'gallery', 'архів', 'роботи'], href: '/figurines', label: ['Все работы', 'Усі роботи', 'All works'] },
];

const WORK_WORDS = ['работ', 'фигур', 'кукл', 'work', 'figurin', 'doll', 'робот', 'фігур', 'ляльк'];
const TALE_WORDS = ['небыл', 'байк', 'tale', 'story', 'небил', 'истори'];
const LATEST_WORDS = ['последн', 'новую', 'новый', 'новое', 'свежую', 'свежий', 'latest', 'newest', 'остан', 'нову'];
const RANDOM_WORDS = ['случайн', 'наугад', 'random', 'навмання', 'любую', 'будь-як'];
const ARCHIVE_PHRASES = ['в архиве', 'в архив', 'in the archive', 'in archive', 'в архіві', 'в архів'];
const ARCHIVE_NOISE = new Set(['архив', 'архиве', 'архів', 'архіві', 'archive', 'найди', 'найти', 'покажи', 'показать', 'открой', 'открыть', 'show', 'find', 'open', 'знайди', 'відкрий']);

export function parseGo(query: string): GoSpec | null {
  const q = normalizeQuery(query);
  const tokens = tokensOf(q);
  const has = (phrases: string[]) => phrases.some((p) => q.includes(p));
  const hasStem = (stems: string[]) => tokens.some((t) => startsWithAny(t, stems));

  // Язык: нужен глагол «переключи» и название языка, иначе это поиск («куклы на английском»).
  if (has(SWITCH_VERBS)) {
    for (const [lang, stems] of LANG_STEMS) {
      if (hasStem(stems)) return { kind: 'lang', lang };
    }
  }

  if (has(ARCHIVE_PHRASES)) {
    const topic = tokens.filter((t) => !FILLER.has(t) && !ARCHIVE_NOISE.has(t)).join(' ');
    if (topic) return { kind: 'archive', topic };
  }

  const verb = has(VERBS);
  const asksWork = hasStem(WORK_WORDS);
  const asksTale = hasStem(TALE_WORDS);

  if (hasStem(RANDOM_WORDS) && (asksWork || asksTale)) return { kind: 'random', what: asksTale ? 'tale' : 'work' };
  if (hasStem(LATEST_WORDS) && (asksWork || asksTale) && (verb || has(['покажи', 'show']))) {
    return { kind: 'latest', what: asksTale ? 'tale' : 'work' };
  }

  if (verb) {
    for (const p of PAGES) {
      if (has(p.stems) || hasStem(p.stems)) return { kind: 'page', href: p.href, label: p.label };
    }
  }
  return null;
}

export function goReply(index: AskIndex, lang: Lang, spec: GoSpec): AskReply {
  const open = (label: string, href: string, hint: string): AskReply => ({
    intent: 'go',
    paragraphs: [say(lang, `Открываю: ${quote(label)}.`, `Відкриваю: ${quote(label)}.`, `Opening ${quote(label)}.`)],
    links: [{ kind: 'section', label, hint, href }],
    action: { kind: 'go', href },
  });
  const arrow = say(lang, 'перейти', 'перейти', 'go');

  switch (spec.kind) {
    case 'page': {
      const label = say(lang, ...spec.label);
      return open(label, spec.href, arrow);
    }
    case 'archive': {
      const href = `/figurines?q=${encodeURIComponent(spec.topic)}`;
      return {
        intent: 'go',
        paragraphs: [say(lang, `Ищу ${quote(spec.topic)} в архиве работ.`, `Шукаю ${quote(spec.topic)} в архіві робіт.`, `Searching the archive for ${quote(spec.topic)}.`)],
        links: [{ kind: 'section', label: say(lang, 'Архив работ', 'Архів робіт', 'The archive'), hint: spec.topic, href }],
        action: { kind: 'go', href },
      };
    }
    case 'latest': {
      if (spec.what === 'tale') {
        const tale = talesNewest(index)[0];
        if (tale) return open(leafTitle(tale, lang), leafHref(tale, 'site_ask'), say(lang, 'новая небылица', 'нова небилиця', 'newest tall tale'));
        return open(say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), '/tales', arrow);
      }
      const work = newest(index.works, (w) => w.createdAt)[0];
      if (work) return open(workName(work), figurineHref(work, 'site_ask'), say(lang, 'новая работа', 'нова робота', 'newest work'));
      return open(say(lang, 'Все работы', 'Усі роботи', 'All works'), '/figurines', arrow);
    }
    case 'random': {
      if (spec.what === 'tale') {
        const tales = index.tales.filter((l) => leafCopy(l, lang).body);
        const pick = tales[Math.floor(Math.random() * tales.length)];
        if (pick) return open(leafTitle(pick, lang), leafHref(pick, 'site_ask'), say(lang, 'случайная небылица', 'випадкова небилиця', 'random tall tale'));
        return open(say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), '/tales', arrow);
      }
      const works = index.works;
      const pick = works[Math.floor(Math.random() * works.length)];
      if (pick) return open(workName(pick), figurineHref(pick, 'site_ask'), say(lang, 'случайная работа', 'випадкова робота', 'random work'));
      return open(say(lang, 'Все работы', 'Усі роботи', 'All works'), '/figurines', arrow);
    }
    case 'lang': {
      // Подтверждение — на том языке, на который переключились: его и увидят дальше.
      const text = {
        en: `Switched to English.`,
        ru: `Переключил на русский.`,
        uk: `Перемкнув на українську.`,
      }[spec.lang];
      return {
        intent: 'go',
        paragraphs: [text],
        links: [],
        action: { kind: 'lang', lang: spec.lang } satisfies AskAction,
      };
    }
  }
}
