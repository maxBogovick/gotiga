// Вопросы окна «Спросить сайт», когда их много: какие показать в пустом поле,
// как разложить по темам и какие подсказать, пока человек печатает. Чистые
// функции над списком `GET /ask/questions`; сам список и подписи тем лежат в
// другом месте (сервер и словарь страницы).
import type { AskQuestion } from '$lib/types/api';

/** Темы в том порядке, в каком они показываются. Те же, что `ask::TOPICS` на сервере. */
export const ASK_TOPICS = ['works', 'order', 'rights', 'site', 'stories', 'play', 'studio'] as const;
export type AskTopic = (typeof ASK_TOPICS)[number];

/** Ключи подписей тем в словаре страницы. */
export const TOPIC_LABEL = {
  works: 'askTopicWorks',
  order: 'askTopicOrder',
  rights: 'askTopicRights',
  site: 'askTopicSite',
  stories: 'askTopicStories',
  play: 'askTopicPlay',
  studio: 'askTopicStudio',
} as const;

export type AskPage = 'work' | 'tale' | 'play' | 'gazette';

/** Страница, на которой стоит человек, как её называет каталог. */
export function pageKeyOf(pathname: string): AskPage | null {
  if (/^\/figurines\/[^/]+(?:\/passport)?\/?$/.test(pathname)) return 'work';
  if (pathname === '/tales' || pathname.startsWith('/tales/')) return 'tale';
  if (pathname === '/battles' || pathname.startsWith('/battles/')) return 'play';
  if (pathname === '/gazette' || pathname.startsWith('/gazette/')) return 'gazette';
  return null;
}

function topicOf(q: AskQuestion): AskTopic {
  return (ASK_TOPICS as readonly string[]).includes(q.topic) ? (q.topic as AskTopic) : 'site';
}

/** Вопросы, которые каталог отметил для этой страницы, в порядке каталога. */
export function forPage(list: AskQuestion[], page: AskPage, max = 3): AskQuestion[] {
  return list.filter((q) => q.pages.includes(page)).slice(0, max);
}

export function inTopic(list: AskQuestion[], topic: AskTopic): AskQuestion[] {
  return list.filter((q) => topicOf(q) === topic);
}

/** Темы, в которых есть хоть один вопрос: пустую тему не показываем. */
export function topicsWithQuestions(list: AskQuestion[]): AskTopic[] {
  const have = new Set(list.map(topicOf));
  return ASK_TOPICS.filter((t) => have.has(t));
}

/** Все вопросы по темам, с местом первого вопроса темы в общем списке: по нему
 *  строки списка находят свои номера для стрелок. */
export function groupByTopic(list: AskQuestion[]): { topic: AskTopic; items: AskQuestion[]; at: number }[] {
  let at = 0;
  return topicsWithQuestions(list).map((topic) => {
    const items = inTopic(list, topic);
    const group = { topic, items, at };
    at += items.length;
    return group;
  });
}

const words = (text: string): string[] =>
  text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean);

/** Слово запроса подходит слову фразы, если у них общее начало: «достав» — «доставка»,
 *  «доставить». Основа берётся по длине запроса, а не по правилам языка: их шесть. */
function alike(typed: string, word: string): boolean {
  if (word.startsWith(typed)) return true;
  let n = 0;
  while (n < typed.length && n < word.length && typed[n] === word[n]) n += 1;
  return typed.length >= 4 && n >= Math.min(typed.length, 5);
}

/** Какие вопросы подсказать, пока человек печатает слово или фразу. Каждое
 *  значимое слово запроса (от четырёх знаков) должно найтись в одной
 *  формулировке: так «достав» не тянет за собой всё, где есть «как».
 *  Только что напечатанный целиком вопрос не предлагается: он уже задан. */
export function matchQuestions(list: AskQuestion[], query: string, max = 3): AskQuestion[] {
  const typed = words(query);
  if (query.trim().length < 3) return [];
  let need = typed.filter((w) => w.length >= 4);
  // Одно короткое слово («бой», «цена») — тоже запрос, но только оно одно.
  if (!need.length) {
    if (typed.length !== 1 || typed[0].length < 3) return [];
    need = typed;
  }
  const same = typed.join(' ');
  const scored: { q: AskQuestion; score: number; at: number }[] = [];
  list.forEach((q, at) => {
    if (words(q.question).join(' ') === same) return;
    let best = Infinity;
    for (const phrase of [q.question, ...q.phrases]) {
      const own = words(phrase);
      if (need.every((w) => own.some((o) => alike(w, o)))) {
        // Меньше слов во фразе — запрос ближе к ней; напечатанный вопрос чуть впереди.
        best = Math.min(best, own.length + (phrase === q.question ? 0 : 0.5));
      }
    }
    if (best < Infinity) scored.push({ q, score: best, at });
  });
  scored.sort((a, b) => a.score - b.score || a.at - b.at);
  return scored.slice(0, max).map((x) => x.q);
}
