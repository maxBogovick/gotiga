// «Спросить сайт»: ответ на вопрос обычными словами из данных самого сайта.
//
// Нейросети здесь нет. Вопрос разбирается по словам-приметам на один из шести
// видов (сводка, с чего начать, что нового, что почитать, во что сыграть, поиск),
// а ответ собирается шаблоном из чисел и названий, которые уже лежат в
// `AskIndex`. Поиск по теме — тот же `searchSite`, что в шапке, поэтому
// «ведьма» здесь и там находит одно и то же.
//
// Только сопоставление и сборка текста: без сети и без хранилищ, чтобы это
// можно было проверить отдельно. Данные загружает `site-index`, смысловой
// поиск сервера спрашивает стор `site-ask`.
import { normalizeQuery } from '$lib/keeper-search';
import { pageReply, wantsPage, type AskPage } from '$lib/site-ask-page';
import { goReply, parseGo, type GoSpec } from '$lib/site-ask-go';
import { leafCopy } from '$lib/gazette';
import { readingMinutes } from '$lib/tales';
import { cardRace, cardTitle, searchSite } from '$lib/site-search';
import type { AskIndex } from '$lib/site-index';
import type { BattleCard, FigurineListItem } from '$lib/types/api';
import {
  say, amount, joinList, quote, short_min, n_works, n_tales, n_leaves, n_cards, n_etudes, n_minutes,
  workName, leafTitle, etudeTitle, newest, heroCards, etudesInOrder, talesNewest,
  workLink, taleLink, leafLink, cardLink, etudeLink, LINKS_MAX, SRC,
  FILLER, tokensOf, startsWithAny, MATERIALS, materialsOf, heightsOf,
  type AskIntent, type AskLink, type AskLinkKind, type AskReply, type Lang,
} from '$lib/site-ask-kit';

export type { AskIntent, AskLink, AskLinkKind, AskReply };

// ── Разбор вопроса ─────────────────────────────────────────────────────────

const SUMMARY_PHRASES = [
  'суммар', 'коротко', 'кратко', 'о сайте', 'про сайт', 'что за сайт', 'что это за сайт', 'что это такое',
  'что здесь', 'что тут', 'что на сайте', 'о чем сайт', 'о чем этот', 'обзор', 'расскажи о себе',
  'підсумуй', 'стисло', 'що це за сайт', 'що тут', 'що на сайті', 'про що сайт', 'огляд',
  'summar', 'overview', 'briefly', 'about the site', 'about this site', 'what is this', 'what is here', "what's here", 'tell me about',
];
const START_PHRASES = [
  'с чего начать', 'с чего начинать', 'что посмотреть', 'посоветуй', 'порекомендуй', 'что выбрать', 'что выбрать',
  'з чого почати', 'що подивитися', 'порадь', 'порекомендуй',
  'where to start', 'where do i start', 'what should i', 'recommend', 'suggest',
];
const NEW_STEMS = ['нов', 'свеж', 'последн', 'останн', 'new', 'latest', 'recent', 'fresh'];
const TALE_STEMS = ['почита', 'почит', 'байк', 'небыл', 'небил', 'рассказ', 'сказк', 'казк', 'истори', 'історі', 'оповід', 'tale', 'story', 'stories', 'read'];
const PLAY_STEMS = ['игр', 'сыгра', 'играт', 'этюд', 'бой', 'боя', 'битв', 'гра', 'зіграт', 'грати', 'етюд', 'бій', 'game', 'play', 'etude', 'study', 'battle', 'карт', 'card'];


export type AboutKey = 'author' | 'order' | 'contact' | 'made' | 'rules' | 'tale' | 'gazette';
export type FactKey = 'works' | 'inprogress' | 'tales' | 'leaves' | 'cards' | 'etudes' | 'races' | 'earliest';

export interface AskParsed {
  intent: AskIntent;
  /** Тема вопроса без служебных слов; пусто, если спрашивают вообще. */
  topic: string;
  /** «на 5 минут»: сколько минут чтения у человека. */
  minutes: number | null;
  /** Про что спрашивают дом и автора (`about`). */
  about?: AboutKey;
  /** Что считают (`count`); пусто — всё сразу. */
  facts?: FactKey[];
  /** Спросили цену: ответ начинается с того, что цен на сайте нет. */
  price?: boolean;
  /** Команда: куда перейти или что переключить. */
  go?: GoSpec;
}

const ABOUT_PHRASES: [AboutKey, string[]][] = [
  ['author', ['кто автор', 'кто делает', 'кто делал', 'кто создает', 'кто создал', 'кто мастер', 'кто вы', 'кто ты', 'об авторе', 'про автора',
    'who makes', 'who made', 'who is the author', 'who are you', 'about the author',
    'хто автор', 'хто робить', 'хто ви', 'хто ти', 'про авторку']],
  ['order', ['как заказать', 'как купить', 'как приобрести', 'как получить работу', 'как запросить', 'заказать работу', 'купить работу', 'хочу купить', 'хочу заказать',
    'доставк', 'оплат', 'депозит', 'сколько стоит', 'цена', 'цены', 'стоимость',
    'how to order', 'how to buy', 'how can i buy', 'how to acquire', 'commission', 'delivery', 'payment', 'how much', 'price',
    'як замовити', 'як купити', 'як придбати', 'скільки коштує', 'ціна', 'ціни']],
  ['contact', ['как связаться', 'как написать', 'написать автору', 'связаться с', 'контакт',
    'how to contact', 'contact the author', 'get in touch', 'як зв’язатися', 'як звязатися', 'як написати', 'зв’язатися', 'зв’язок']],
  ['made', ['из чего', 'из какого материала', 'материал', 'как сделан', 'как делаются', 'как лепит', 'какого размера', 'какой размер', 'размер',
    'made of', 'what material', 'how are they made', 'how is it made', 'techniq', 'what size', 'how big',
    'з чого', 'з якого матеріалу', 'як зроблен', 'матеріал', 'якого розміру', 'розмір']],
  ['rules', ['как играть', 'правила', 'что такое этюд', 'что такое игра', 'как работает игра', 'how to play', 'rules', 'what is a study', 'how does the game',
    'як грати', 'що таке етюд', 'що таке гра']],
  ['tale', ['что такое небылиц', 'что такое байк', 'what is a tall tale', 'what are tall tales', 'що таке небилиц']],
  ['gazette', ['что такое вестник', 'what is the gazette', 'що таке вісник']],
];
const PRICE_WORDS = ['сколько стоит', 'цена', 'цены', 'стоимость', 'how much', 'price', 'скільки коштує', 'ціна', 'ціни'];

const COUNT_TRIGGERS = ['сколько', 'скільки', 'how many', 'количество', 'кількість'];
const FACT_STEMS: [FactKey, string[]][] = [
  ['tales', ['небыл', 'небил', 'tale', 'байк', 'истори', 'історі']],
  ['leaves', ['вестник', 'вісник', 'gazette', 'лист']],
  ['cards', ['карт', 'card']],
  ['etudes', ['этюд', 'етюд', 'stud', 'etude']],
  ['races', ['расы', 'расам', 'расах', 'раси', 'race']],
  ['works', ['работ', 'робіт', 'робот', 'work', 'figurin', 'кукл', 'фигур', 'ляльк', 'фігур']],
];
const IN_PROGRESS = ['в работе', 'у роботі', 'in progress', 'в процессе', 'у процесі'];
const EARLIEST = ['самая ранн', 'самая стар', 'самая перв', 'найраніш', 'найстарш', 'перша робота', 'earliest', 'oldest', 'first work', 'с какого года', 'з якого року', 'since when'];


export function parseAsk(query: string): AskParsed {
  const q = normalizeQuery(query);
  const tokens = tokensOf(q);
  const minutesMatch = q.match(/(\d{1,3})\s*(?:мин|хв|min)/);
  const minutes = minutesMatch ? Number(minutesMatch[1]) : null;
  const has = (phrases: string[]) => phrases.some((p) => q.includes(p));

  const topicWithout = (stems: string[]) =>
    tokens
      .filter((t) => !startsWithAny(t, stems))
      .filter((t) => !FILLER.has(t) && !/^\d+$/.test(t) && !/^(?:мин|хв|min)/.test(t))
      .join(' ');

  if (!q) return { intent: 'none', topic: '', minutes };
  if (wantsPage(query)) return { intent: 'page', topic: '', minutes };
  if (has(SUMMARY_PHRASES)) return { intent: 'summary', topic: '', minutes };
  if (has(START_PHRASES)) return { intent: 'start', topic: '', minutes };

  const go = parseGo(query);
  if (go) return { intent: 'go', topic: '', minutes, go };

  for (const [about, phrases] of ABOUT_PHRASES) {
    if (has(phrases)) return { intent: 'about', topic: '', minutes, about, price: has(PRICE_WORDS) };
  }

  // «Сколько …» и «самая ранняя работа»: ответ — число или название из данных.
  const earliestAsked = has(EARLIEST);
  if (has(COUNT_TRIGGERS) || earliestAsked) {
    const facts: FactKey[] = [];
    if (earliestAsked) facts.push('earliest');
    // «Самая ранняя работа» — не счёт: слово «работа» там не просит число работ.
    if (has(COUNT_TRIGGERS)) {
      if (has(IN_PROGRESS)) facts.push('inprogress');
      const rest = tokens.filter((t) => !['работе', 'роботі'].includes(t));
      for (const [key, stems] of FACT_STEMS) {
        if (rest.some((t) => startsWithAny(t, stems) || (key === 'races' && t === 'рас')) && !facts.includes(key)) facts.push(key);
      }
    }
    return { intent: 'count', topic: '', minutes, facts };
  }

  // «Новое» — только если за ним нет темы: «новые ведьмы» ищет ведьм.
  const newTopic = topicWithout(NEW_STEMS);
  if (tokens.some((t) => t.length <= 8 && startsWithAny(t, NEW_STEMS)) && !newTopic) {
    return { intent: 'new', topic: '', minutes };
  }
  if (tokens.some((t) => startsWithAny(t, TALE_STEMS))) {
    return { intent: 'tale', topic: topicWithout(TALE_STEMS), minutes };
  }
  if (tokens.some((t) => startsWithAny(t, PLAY_STEMS))) {
    return { intent: 'play', topic: topicWithout(PLAY_STEMS), minutes };
  }
  const topic = topicWithout([]);
  return { intent: topic ? 'search' : 'none', topic, minutes };
}

// ── Ответы ─────────────────────────────────────────────────────────────────

function summaryReply(index: AskIndex, lang: Lang, gazetteTotal: number | null): AskReply {
  const works = index.works;
  const inProgress = works.filter((w) => w.status === 'in_progress').length;
  const years = works.map((w) => w.year).filter((y): y is number => typeof y === 'number' && y > 0);
  const earliest = years.length ? Math.min(...years) : null;
  const cards = heroCards(index);
  const etudes = etudesInOrder(index);
  const leaves = gazetteTotal ?? index.gazette.length;

  const first =
    say(lang, 'Это витрина авторских готических миниатюр, а не магазин. ', 'Це вітрина авторських готичних мініатюр, а не магазин. ', 'This is a showcase of original gothic miniatures, not a shop. ') +
    (works.length
      ? say(lang, `В собрании ${n_works(lang, works.length)}`, `У зібранні ${n_works(lang, works.length)}`, `The collection holds ${n_works(lang, works.length)}`) +
        (inProgress
          ? say(lang, `, из них ${inProgress} в работе`, `, з них ${inProgress} у роботі`, `, ${inProgress} of them still in progress`)
          : '') +
        (earliest
          ? say(lang, `; самая ранняя — ${earliest} года.`, `; найраніша — ${earliest} року.`, `; the earliest is from ${earliest}.`)
          : '.')
      : '');

  const beside: string[] = [];
  if (index.tales.length) beside.push(n_tales(lang, index.tales.length));
  if (leaves) beside.push(n_leaves(lang, leaves));
  if (cards.length) beside.push(say(lang, `${n_cards(lang, cards.length)} в игре`, `${n_cards(lang, cards.length)} у грі`, `${n_cards(lang, cards.length)} in the game`));
  if (etudes.length) {
    beside.push(
      say(lang, `${n_etudes(lang, etudes.length)} — партии против компьютера`, `${n_etudes(lang, etudes.length)} — партії проти комп’ютера`, `${n_etudes(lang, etudes.length)} — games against the computer`),
    );
  }
  const second = beside.length
    ? say(lang, 'Рядом с работами: ', 'Поряд із роботами: ', 'Alongside the works: ') + joinList(lang, beside) + '.'
    : '';

  const races = new Map<string, number>();
  for (const c of cards) {
    const r = cardRace(c, lang);
    if (r) races.set(r, (races.get(r) ?? 0) + 1);
  }
  const topRaces = [...races.entries()].filter(([, n]) => n >= 2).sort((a, b) => b[1] - a[1]).slice(0, 3).map(([r]) => r);
  const third = topRaces.length >= 2
    ? say(lang, `Чаще всего среди карт: ${topRaces.join(', ')}.`, `Найчастіше серед карт: ${topRaces.join(', ')}.`, `Most common among the cards: ${topRaces.join(', ')}.`)
    : '';

  const latestWork = newest(works, (w) => w.createdAt)[0];
  const latestTale = talesNewest(index)[0];
  const lastBits: string[] = [];
  if (latestWork) lastBits.push(say(lang, `работа ${quote(workName(latestWork))}`, `робота ${quote(workName(latestWork))}`, `the work ${quote(workName(latestWork))}`));
  if (latestTale) lastBits.push(say(lang, `небылица ${quote(leafTitle(latestTale, lang))}`, `небилиця ${quote(leafTitle(latestTale, lang))}`, `the tall tale ${quote(leafTitle(latestTale, lang))}`));
  const fourth = lastBits.length ? say(lang, 'Последнее: ', 'Останнє: ', 'Latest: ') + joinList(lang, lastBits) + '.' : '';

  const links: AskLink[] = [];
  if (latestWork) links.push(workLink(latestWork, lang, say(lang, 'новая работа', 'нова робота', 'newest work')));
  if (latestTale) links.push(taleLink(latestTale, lang));
  links.push({ kind: 'section', label: say(lang, 'Все работы', 'Усі роботи', 'All works'), hint: String(works.length), href: '/figurines' });
  if (index.tales.length) links.push({ kind: 'section', label: say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), hint: String(index.tales.length), href: '/tales' });
  if (leaves) links.push({ kind: 'section', label: say(lang, 'Вестник', 'Вісник', 'Gazette'), hint: String(leaves), href: '/gazette' });
  const firstEtude = etudes[0];
  links.push(
    firstEtude
      ? { kind: 'section', label: say(lang, 'Игра', 'Гра', 'The game'), hint: etudeTitle(firstEtude, lang), href: '/battles/etude' }
      : { kind: 'section', label: say(lang, 'Игра', 'Гра', 'The game'), href: '/battles' },
  );

  return { intent: 'summary', paragraphs: [first, second, third, fourth].filter(Boolean), links: links.slice(0, LINKS_MAX) };
}

function startReply(index: AskIndex, lang: Lang): AskReply {
  const worksById = new Map(index.works.map((w) => [w.id, w]));
  const cardOf = new Map(heroCards(index).filter((c) => c.figurineId).map((c) => [c.figurineId as string, c]));
  const tales = talesNewest(index).filter((l) => readingMinutes(leafCopy(l, lang).body) > 0);

  const talesWithWork = tales.filter((l) => l.figurineId && worksById.has(l.figurineId));
  const full = talesWithWork.find((l) => cardOf.has(l.figurineId as string));
  const tale = full ?? talesWithWork[0] ?? tales[0] ?? null;
  const work = tale
    ? (tale.figurineId ? worksById.get(tale.figurineId) ?? null : null)
    : (newest(index.works, (w) => w.createdAt)[0] ?? null);
  const card = work ? cardOf.get(work.id) ?? null : null;

  if (!work && !tale) {
    return {
      intent: 'start',
      paragraphs: [say(lang, 'Пока на сайте нечего посоветовать: попробуйте открыть архив работ.', 'Поки на сайті нічого порадити: спробуйте відкрити архів робіт.', 'There is nothing to suggest yet: try the archive of works.')],
      links: [{ kind: 'section', label: say(lang, 'Все работы', 'Усі роботи', 'All works'), href: '/figurines' }],
    };
  }

  const links: AskLink[] = [];
  if (work) links.push(workLink(work, lang));
  if (tale) links.push(taleLink(tale, lang));
  if (card) links.push(cardLink(card, lang));

  const parts: string[] = [];
  if (tale) {
    const min = readingMinutes(leafCopy(tale, lang).body);
    parts.push(say(lang, `небылица ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `небилиця ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `the tall tale ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`));
  }
  if (card) parts.push(say(lang, 'карта в игре', 'карта у грі', 'a card in the game'));

  const lead = work
    ? say(lang, `Начните с работы ${quote(workName(work))}`, `Почніть із роботи ${quote(workName(work))}`, `Start with the work ${quote(workName(work))}`)
    : say(lang, 'Начните с небылицы', 'Почніть із небилиці', 'Start with a tall tale');
  const body = parts.length
    ? say(lang, `: о ней есть ${joinList(lang, parts)}.`, `: про неї є ${joinList(lang, parts)}.`, `: it comes with ${joinList(lang, parts)}.`)
    : '.';
  const order = say(
    lang,
    'Порядок простой: посмотреть, прочитать, сыграть.',
    'Порядок простий: подивитися, прочитати, зіграти.',
    'The order is simple: look, read, play.',
  );
  return { intent: 'start', paragraphs: [lead + body, order], links };
}

function newReply(index: AskIndex, lang: Lang): AskReply {
  const works = newest(index.works, (w) => w.createdAt).slice(0, 3);
  const tale = talesNewest(index)[0] ?? null;
  const leaf = newest(index.gazette, (l) => l.publishedAt ?? l.createdAt)[0] ?? null;

  const names = works.map((w) => quote(workName(w)));
  const sentences: string[] = [];
  if (names.length) {
    sentences.push(say(lang, `Недавно в собрании появились: ${joinList(lang, names)}.`, `Нещодавно в зібранні з’явилися: ${joinList(lang, names)}.`, `Recently added to the collection: ${joinList(lang, names)}.`));
  }
  const reading: string[] = [];
  if (tale) reading.push(say(lang, `небылица ${quote(leafTitle(tale, lang))}`, `небилиця ${quote(leafTitle(tale, lang))}`, `the tall tale ${quote(leafTitle(tale, lang))}`));
  if (leaf) reading.push(say(lang, `в вестнике — ${quote(leafTitle(leaf, lang))}`, `у віснику — ${quote(leafTitle(leaf, lang))}`, `in the gazette ${quote(leafTitle(leaf, lang))}`));
  if (reading.length) {
    sentences.push(say(lang, `Из чтения новее всего: ${joinList(lang, reading)}.`, `З читання найновіше: ${joinList(lang, reading)}.`, `Newest reading: ${joinList(lang, reading)}.`));
  }
  if (!sentences.length) {
    sentences.push(say(lang, 'Пока новых записей нет.', 'Поки нових записів немає.', 'Nothing new yet.'));
  }

  const links: AskLink[] = [...works.map((w) => workLink(w, lang))];
  if (tale) links.push(taleLink(tale, lang));
  if (leaf) links.push(leafLink(leaf, lang));
  return { intent: 'new', paragraphs: sentences, links: links.slice(0, LINKS_MAX) };
}

function taleReply(index: AskIndex, lang: Lang, minutes: number | null): AskReply {
  const timed = index.tales
    .map((l) => ({ l, min: readingMinutes(leafCopy(l, lang).body) }))
    .filter((x) => x.min > 0)
    .sort((a, b) => a.min - b.min || (a.l.shelfOrder ?? 1e9) - (b.l.shelfOrder ?? 1e9));

  if (!timed.length) {
    return {
      intent: 'tale',
      paragraphs: [say(lang, 'Небылиц для чтения пока нет.', 'Небилиць для читання поки немає.', 'There are no tall tales to read yet.')],
      links: [{ kind: 'section', label: say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), href: '/tales' }],
    };
  }

  const fits = minutes ? timed.filter((x) => x.min <= minutes) : [];
  const shown = (fits.length ? fits : timed).slice(0, 3);

  let text: string;
  if (minutes && fits.length) {
    text = say(
      lang,
      `На ${n_minutes(lang, minutes)} подойдёт: ${joinList(lang, shown.map((x) => quote(leafTitle(x.l, lang))))}.`,
      `На ${n_minutes(lang, minutes)} підійде: ${joinList(lang, shown.map((x) => quote(leafTitle(x.l, lang))))}.`,
      `For ${n_minutes(lang, minutes)}, try ${joinList(lang, shown.map((x) => quote(leafTitle(x.l, lang))))}.`,
    );
  } else if (minutes) {
    text = say(
      lang,
      `Короче ${n_minutes(lang, minutes)} небылиц нет. Самая короткая — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} чтения.`,
      `Коротших за ${n_minutes(lang, minutes)} небилиць немає. Найкоротша — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} читання.`,
      `There is no tall tale shorter than ${n_minutes(lang, minutes)}. The shortest is ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} to read.`,
    );
  } else {
    text = say(
      lang,
      `На полке ${n_tales(lang, index.tales.length)}. Самая короткая — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} чтения.`,
      `На полиці ${n_tales(lang, index.tales.length)}. Найкоротша — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} читання.`,
      `The shelf holds ${n_tales(lang, index.tales.length)}. The shortest is ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} to read.`,
    );
  }
  const links = shown.map((x) => taleLink(x.l, lang));
  links.push({ kind: 'section', label: say(lang, 'Все небылицы', 'Усі небилиці', 'All tall tales'), hint: String(index.tales.length), href: '/tales' });
  return { intent: 'tale', paragraphs: [text], links };
}

function playReply(index: AskIndex, lang: Lang): AskReply {
  const cards = heroCards(index);
  const etudes = etudesInOrder(index);
  const first = etudes[0];

  const paragraphs = [
    say(
      lang,
      `В игре ${n_cards(lang, cards.length)}. Этюд — это партия против компьютера: вы расставляете карты и играете.`,
      `У грі ${n_cards(lang, cards.length)}. Етюд — це партія проти комп’ютера: ви розставляєте карти й граєте.`,
      `The game has ${n_cards(lang, cards.length)}. A study is a game against the computer: you place your cards and play.`,
    ),
  ];
  if (first) {
    paragraphs.push(
      say(
        lang,
        `Сейчас ${n_etudes(lang, etudes.length)}; первый — ${quote(etudeTitle(first, lang))}. Карты можно смотреть и без игры — на полке.`,
        `Зараз ${n_etudes(lang, etudes.length)}; перший — ${quote(etudeTitle(first, lang))}. Карти можна дивитися й без гри — на полиці.`,
        `There ${etudes.length === 1 ? 'is' : 'are'} ${n_etudes(lang, etudes.length)} now; the first is ${quote(etudeTitle(first, lang))}. You can also just look at the cards on the shelf.`,
      ),
    );
  }
  const links: AskLink[] = [];
  if (first) links.push(etudeLink(first, lang));
  links.push({ kind: 'section', label: say(lang, 'Полка карт', 'Полиця карт', 'Card shelf'), hint: String(cards.length), href: '/battles' });
  if (etudes.length > 1) links.push({ kind: 'section', label: say(lang, 'Все этюды', 'Усі етюди', 'All studies'), hint: String(etudes.length), href: '/battles/etude' });
  return { intent: 'play', paragraphs, links };
}

/** Вопросы, которые сайт умеет: из них «не умею» предлагает ближайшие к сказанному. */
function examples(lang: Lang): string[] {
  return [
    say(lang, 'Коротко: что это за сайт?', 'Коротко: що це за сайт?', 'Briefly, what is this site?'),
    say(lang, 'Сколько работ на сайте?', 'Скільки робіт на сайті?', 'How many works are on the site?'),
    say(lang, 'Что нового?', 'Що нового?', 'What is new?'),
    say(lang, 'Что почитать за 5 минут?', 'Що почитати за 5 хвилин?', 'A tale to read in 5 minutes'),
    say(lang, 'С чего начать?', 'З чого почати?', 'Where to start?'),
    say(lang, 'Во что можно сыграть?', 'У що можна зіграти?', 'What can I play?'),
    say(lang, 'Как играть?', 'Як грати?', 'How to play?'),
    say(lang, 'Как заказать работу?', 'Як замовити роботу?', 'How to order a work?'),
    say(lang, 'Из чего сделаны фигурки?', 'З чого зроблені фігурки?', 'What are the figurines made of?'),
    say(lang, 'Кто автор?', 'Хто автор?', 'Who is the author?'),
    say(lang, 'Как связаться с автором?', 'Як зв’язатися з автором?', 'How to contact the author?'),
    say(lang, 'Какая самая ранняя работа?', 'Яка найраніша робота?', 'Which is the earliest work?'),
    say(lang, 'Покажи ведьм', 'Покажи відьом', 'Show me a witch'),
    say(lang, 'Сколько рас в игре?', 'Скільки рас у грі?', 'How many races are in the game?'),
  ];
}

/** Три вопроса из тех, что сайт умеет, ближайшие к сказанному по началам слов. */
function nearestQuestions(lang: Lang, query: string): AskLink[] {
  const stems = (text: string) =>
    new Set(
      tokensOf(normalizeQuery(text))
        .filter((t) => t.length >= 3 && !FILLER.has(t))
        .map((t) => t.slice(0, 4)),
    );
  const asked = stems(query);
  const ranked = examples(lang)
    .map((q, i) => {
      const own = stems(q);
      let hits = 0;
      for (const w of own) if (asked.has(w)) hits += 1;
      return { q, i, hits };
    })
    .sort((a, b) => b.hits - a.hits || a.i - b.i);
  const picked = ranked.filter((x) => x.hits > 0).slice(0, 3);
  // Совпадений нет — три самых обычных вопроса.
  const rows = picked.length ? picked : ranked.filter((x) => [0, 1, 3].includes(x.i));
  return rows.map(({ q }) => ({ kind: 'question' as const, label: q, href: '' }));
}

function noneReply(lang: Lang, topic: string, query: string): AskReply {
  const paragraphs = topic
    ? [
        say(
          lang,
          `По запросу ${quote(topic)} ничего не нашлось. Попробуйте назвать героиню или материал («ведьма», «гном», «шерсть») или спросите так:`,
          `За запитом ${quote(topic)} нічого не знайшлося. Спробуйте назвати героїню або матеріал («відьма», «гном», «вовна») чи запитайте так:`,
          `Nothing found for ${quote(topic)}. Try a character or a material (“witch”, “gnome”, “wool”), or ask like this:`,
        ),
      ]
    : [
        say(
          lang,
          'Такой вопрос я пока не умею. Я отвечаю по данным самого сайта: рассказываю о нём, считаю работы, небылицы и карты, ищу по темам, подсказываю, что почитать и во что сыграть. Например:',
          'Таке питання я поки не вмію. Я відповідаю за даними самого сайту: розповідаю про нього, рахую роботи, небилиці й карти, шукаю за темами, підказую, що почитати й у що зіграти. Наприклад:',
          'I cannot answer that yet. I answer from the site’s own data: I describe it, count works, tall tales and cards, search by topic, and suggest what to read and play. For example:',
        ),
      ];
  return { intent: 'none', missed: true, paragraphs, links: nearestQuestions(lang, query) };
}

type SearchScopeAsk = 'all' | 'tales' | 'cards';

function searchReply(
  index: AskIndex,
  lang: Lang,
  topic: string,
  meaning: string[] | null,
  scope: SearchScopeAsk,
  intent: AskIntent,
): AskReply {
  const found = searchSite(index, topic, lang, meaning);
  const works = scope === 'all' ? found?.works ?? [] : [];
  const tales = scope === 'cards' ? [] : found?.tales ?? [];
  const leaves = scope === 'all' ? found?.gazette ?? [] : [];
  const cards = scope === 'tales' ? [] : found?.cards ?? [];

  if (!found || works.length + tales.length + leaves.length + cards.length === 0) {
    return { ...noneReply(lang, topic, topic), intent };
  }

  // Одно «близко по смыслу» без точных совпадений — это не ответ: смысловой поиск
  // находит ближайшую работу к чему угодно, и «нашлось» было бы неправдой.
  const exact = works.filter((f) => f.why.kind !== 'meaning').length + tales.length + leaves.length + cards.length;
  if (exact === 0) {
    const near = works.slice(0, 3);
    return {
      intent,
      missed: true,
      paragraphs: [
        say(
          lang,
          `Точных совпадений по запросу ${quote(topic)} нет. Ближе всего по смыслу: ${joinList(lang, near.map((f) => quote(workName(f.item))))}. Если это не то, назовите героиню или материал.`,
          `Точних збігів за запитом ${quote(topic)} немає. Найближче за змістом: ${joinList(lang, near.map((f) => quote(workName(f.item))))}. Якщо це не те, назвіть героїню або матеріал.`,
          `No exact match for ${quote(topic)}. Closest in meaning: ${joinList(lang, near.map((f) => quote(workName(f.item))))}. If that is not it, name a character or a material.`,
        ),
      ],
      links: near.map((f) => workLink(f.item, lang, say(lang, 'близко по смыслу', 'близько за змістом', 'close in meaning'))),
    };
  }

  const parts: string[] = [];
  if (works.length) parts.push(n_works(lang, works.length));
  if (tales.length) parts.push(n_tales(lang, tales.length));
  if (leaves.length) parts.push(n_leaves(lang, leaves.length));
  if (cards.length) parts.push(n_cards(lang, cards.length));

  const top = works[0]
    ? say(lang, `работа ${quote(workName(works[0].item))}`, `робота ${quote(workName(works[0].item))}`, `the work ${quote(workName(works[0].item))}`)
    : tales[0]
      ? say(lang, `небылица ${quote(leafTitle(tales[0].item, lang))}`, `небилиця ${quote(leafTitle(tales[0].item, lang))}`, `the tall tale ${quote(leafTitle(tales[0].item, lang))}`)
      : cards[0]
        ? say(lang, `карта ${quote(cardTitle(cards[0].item, lang))}`, `карта ${quote(cardTitle(cards[0].item, lang))}`, `the card ${quote(cardTitle(cards[0].item, lang))}`)
        : say(lang, `лист вестника ${quote(leafTitle(leaves[0].item, lang))}`, `аркуш вісника ${quote(leafTitle(leaves[0].item, lang))}`, `the gazette leaf ${quote(leafTitle(leaves[0].item, lang))}`);

  const paragraphs = [
    say(lang, `По запросу ${quote(topic)} нашлось: ${joinList(lang, parts)}.`, `За запитом ${quote(topic)} знайшлося: ${joinList(lang, parts)}.`, `For ${quote(topic)} I found ${joinList(lang, parts)}.`),
    say(lang, `Ближе всего — ${top}.`, `Найближче — ${top}.`, `Closest: ${top}.`),
  ];

  const links: AskLink[] = [];
  const alone = scope !== 'all';
  for (const f of works.slice(0, 3)) links.push(workLink(f.item, lang));
  for (const f of tales.slice(0, alone ? LINKS_MAX : 2)) links.push(taleLink(f.item, lang));
  for (const f of cards.slice(0, alone ? LINKS_MAX : 2)) links.push(cardLink(f.item, lang));
  for (const f of leaves.slice(0, 1)) links.push(leafLink(f.item, lang));
  return { intent, paragraphs, links: links.slice(0, LINKS_MAX) };
}

function aboutReply(index: AskIndex, lang: Lang, key: AboutKey, price: boolean, text: (k: string) => string): AskReply {
  const sec = (label: string, href: string, hint?: string): AskLink => ({ kind: 'section', label, href, hint });
  switch (key) {
    case 'author':
      return {
        intent: 'about',
        paragraphs: [
          say(
            lang,
            'Все фигурки делает один автор: каждую он лепит, расписывает и одевает вручную, поэтому двух одинаковых нет.',
            'Усі фігурки робить один автор: кожну він ліпить, розписує й одягає вручну, тому двох однакових немає.',
            'Every figurine is made by one author, who sculpts, paints and dresses each by hand, so no two are alike.',
          ),
        ],
        links: [
          sec(say(lang, 'Об авторе', 'Про автора', 'About the author'), '/author'),
          sec(say(lang, 'Мастерская', 'Майстерня', 'The workshop'), '/workshop'),
        ],
      };
    case 'order': {
      const lead = price
        ? say(
            lang,
            'Цен на сайте нет: условия мастер называет лично в переписке. ',
            'Цін на сайті немає: умови майстер називає особисто в листуванні. ',
            'There are no prices on the site: the maker names the terms personally, in a message. ',
          )
        : '';
      return {
        intent: 'about',
        paragraphs: [
          lead + text('acquireStep1Body'),
          say(lang, 'Порядок такой: ', 'Порядок такий: ', 'The order is: ') +
            [text('acquireStep1Title'), text('acquireStep2Title'), text('acquireStep3Title'), text('acquireStep4Title'), text('acquireStep5Title')].join(' → ') + '.',
          text('acquireStep5Body'),
        ],
        links: [
          sec(text('acquireTitle'), '/acquire'),
          sec(text('acquireCommissionCta'), '/commission'),
          sec(text('acquireBrowseCta'), '/figurines'),
        ],
      };
    }
    case 'contact':
      return {
        intent: 'about',
        paragraphs: [
          say(
            lang,
            'Написать автору можно кнопкой «Написать» в шапке или формой на главной странице. Автор читает каждое письмо и отвечает лично, обычно в течение дня.',
            'Написати авторові можна кнопкою «Написати» в шапці або формою на головній сторінці. Автор читає кожен лист і відповідає особисто, зазвичай протягом дня.',
            'You can write to the author with the “Write” button in the header or the form on the home page. The author reads every message and answers personally, usually within a day.',
          ),
        ],
        links: [sec(say(lang, 'Об авторе', 'Про автора', 'About the author'), '/author'), sec(text('acquireTitle'), '/acquire')],
      };
    case 'made': {
      const mats = materialsOf(index, lang);
      const h = heightsOf(index);
      const paragraphs: string[] = [];
      paragraphs.push(
        mats.length
          ? say(
              lang,
              `Основа — ${joinList(lang, mats)}. Фигурки лепят, расписывают и одевают вручную, и каждая существует в единственном экземпляре.`,
              `Основа — ${joinList(lang, mats)}. Фігурки ліплять, розписують і одягають вручну, і кожна існує в єдиному екземплярі.`,
              `The base is ${joinList(lang, mats)}. The figurines are sculpted, painted and dressed by hand, and each exists in a single copy.`,
            )
          : say(
              lang,
              'Фигурки лепят, расписывают и одевают вручную, и каждая существует в единственном экземпляре.',
              'Фігурки ліплять, розписують і одягають вручну, і кожна існує в єдиному екземплярі.',
              'The figurines are sculpted, painted and dressed by hand, and each exists in a single copy.',
            ),
      );
      if (h) {
        const range = h[0] === h[1] ? `${h[0]}` : `${h[0]}–${h[1]}`;
        paragraphs.push(
          say(
            lang,
            `Размер — примерно ${range} см: точный написан на странице каждой работы.`,
            `Розмір — приблизно ${range} см: точний указано на сторінці кожної роботи.`,
            `Size is roughly ${range} cm: the exact figure is on each work’s page.`,
          ),
        );
      }
      return {
        intent: 'about',
        paragraphs,
        links: [sec(say(lang, 'Мастерская', 'Майстерня', 'The workshop'), '/workshop'), sec(say(lang, 'Все работы', 'Усі роботи', 'All works'), '/figurines')],
      };
    }
    case 'rules':
      return {
        intent: 'about',
        paragraphs: [
          say(
            lang,
            'Этюд — партия против компьютера на небольшом поле. Вы расставляете карты и ходите по очереди; выигрывает тот, у кого остались карты, когда у противника их не осталось.',
            'Етюд — партія проти комп’ютера на невеликому полі. Ви розставляєте карти й ходите по черзі; виграє той, у кого залишилися карти, коли в противника їх не залишилося.',
            'A study is a game against the computer on a small field. You place your cards and take turns; whoever still has cards when the opponent has none wins.',
          ),
          say(
            lang,
            'У каждого этюда могут быть свои правила: они написаны на листе боя перед началом. Первые бои можно играть без входа.',
            'У кожного етюду можуть бути свої правила: вони написані на аркуші бою перед початком. Перші бої можна грати без входу.',
            'Each study may have its own rules: they are written on the battle sheet before it starts. The first battles can be played without signing in.',
          ),
        ],
        links: [sec(say(lang, 'Этюды', 'Етюди', 'Studies'), '/battles/etude'), sec(say(lang, 'Полка карт', 'Полиця карт', 'Card shelf'), '/battles')],
      };
    case 'tale':
      return {
        intent: 'about',
        paragraphs: [
          say(
            lang,
            'Небылица — короткая история о фигурке. У многих небылиц есть своя работа, а у работы — карта в игре.',
            'Небилиця — коротка історія про фігурку. У багатьох небилиць є своя робота, а в роботи — карта в грі.',
            'A tall tale is a short story about a figurine. Many have a work of their own, and the work has a card in the game.',
          ),
        ],
        links: [sec(say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), '/tales', String(index.tales.length || ''))],
      };
    case 'gazette':
      return {
        intent: 'about',
        paragraphs: [
          say(
            lang,
            'Вестник — лента коротких листов: о том, что появилось в мастерской, и новости вокруг миниатюр. Небылицы — тоже листы, но у них своя полка.',
            'Вісник — стрічка коротких аркушів: про те, що з’явилося в майстерні, і новини довкола мініатюр. Небилиці — теж аркуші, але в них своя полиця.',
            'The gazette is a feed of short leaves: what has arrived in the workshop, and news around miniatures. Tall tales are leaves too, but they have their own shelf.',
          ),
        ],
        links: [sec(say(lang, 'Вестник', 'Вісник', 'Gazette'), '/gazette'), sec(say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), '/tales')],
      };
  }
}

function countReply(index: AskIndex, lang: Lang, facts: FactKey[], gazetteTotal: number | null): AskReply {
  const works = index.works;
  const cards = heroCards(index);
  const etudes = index.etudes;
  const leaves = gazetteTotal ?? index.gazette.length;
  const inProgress = works.filter((w) => w.status === 'in_progress').length;
  const wanted = facts.length ? facts : (['works', 'tales', 'leaves', 'cards', 'etudes'] as FactKey[]);

  const parts: string[] = [];
  const links: AskLink[] = [];
  const paragraphs: string[] = [];

  for (const f of wanted) {
    switch (f) {
      case 'works':
        parts.push(n_works(lang, works.length));
        links.push({ kind: 'section', label: say(lang, 'Все работы', 'Усі роботи', 'All works'), hint: String(works.length), href: '/figurines' });
        break;
      case 'inprogress':
        parts.push(
          inProgress
            ? say(lang, `${n_works(lang, inProgress)} ещё в работе`, `${n_works(lang, inProgress)} ще в роботі`, `${n_works(lang, inProgress)} still in progress`)
            : say(lang, 'работ «в работе» сейчас нет', 'робіт «у роботі» зараз немає', 'no works “in progress” right now'),
        );
        break;
      case 'tales':
        parts.push(n_tales(lang, index.tales.length));
        links.push({ kind: 'section', label: say(lang, 'Небылицы', 'Небилиці', 'Tall tales'), hint: String(index.tales.length), href: '/tales' });
        break;
      case 'leaves':
        parts.push(n_leaves(lang, leaves));
        links.push({ kind: 'section', label: say(lang, 'Вестник', 'Вісник', 'Gazette'), hint: String(leaves), href: '/gazette' });
        break;
      case 'cards':
        parts.push(say(lang, `${n_cards(lang, cards.length)} в игре`, `${n_cards(lang, cards.length)} у грі`, `${n_cards(lang, cards.length)} in the game`));
        links.push({ kind: 'section', label: say(lang, 'Полка карт', 'Полиця карт', 'Card shelf'), hint: String(cards.length), href: '/battles' });
        break;
      case 'etudes':
        parts.push(n_etudes(lang, etudes.length));
        links.push({ kind: 'section', label: say(lang, 'Этюды', 'Етюди', 'Studies'), hint: String(etudes.length), href: '/battles/etude' });
        break;
      case 'races': {
        const byRace = new Map<string, number>();
        for (const c of cards) {
          const r = cardRace(c, lang);
          if (r) byRace.set(r, (byRace.get(r) ?? 0) + 1);
        }
        const list = [...byRace.entries()].sort((a, b) => b[1] - a[1]);
        if (list.length) {
          paragraphs.push(
            say(
              lang,
              `Рас в игре ${list.length}: ${list.map(([r, n]) => `${r} (${n})`).join(', ')}.`,
              `Рас у грі ${list.length}: ${list.map(([r, n]) => `${r} (${n})`).join(', ')}.`,
              `The game has ${list.length} races: ${list.map(([r, n]) => `${r} (${n})`).join(', ')}.`,
            ),
          );
        } else {
          paragraphs.push(say(lang, 'Расы в игре пока не назначены.', 'Раси в грі поки не призначені.', 'No races are set in the game yet.'));
        }
        links.push({ kind: 'section', label: say(lang, 'Полка карт', 'Полиця карт', 'Card shelf'), href: '/battles' });
        break;
      }
      case 'earliest': {
        const dated = works.filter((w) => typeof w.year === 'number' && (w.year as number) > 0);
        const first = [...dated].sort((a, b) => (a.year as number) - (b.year as number) || (a.createdAt ?? '').localeCompare(b.createdAt ?? ''))[0];
        if (first) {
          paragraphs.push(
            say(
              lang,
              `Самая ранняя работа — ${quote(workName(first))}, ${first.year} года.`,
              `Найраніша робота — ${quote(workName(first))}, ${first.year} року.`,
              `The earliest work is ${quote(workName(first))}, from ${first.year}.`,
            ),
          );
          links.push(workLink(first, lang));
        } else {
          paragraphs.push(say(lang, 'У работ не указан год.', 'У робіт не вказано рік.', 'The works have no year set.'));
        }
        break;
      }
    }
  }

  if (parts.length) {
    paragraphs.unshift(
      say(lang, 'Сейчас на сайте: ', 'Зараз на сайті: ', 'On the site now: ') + joinList(lang, parts) + '.',
    );
  }
  return { intent: 'count', paragraphs, links: links.slice(0, LINKS_MAX) };
}

// ── Вход ───────────────────────────────────────────────────────────────────

/** Нужен ли смысловой поиск сервера: только когда ищется тема среди работ. */
export function needsMeaning(parsed: AskParsed): boolean {
  return parsed.intent === 'search' && parsed.topic.length >= 3;
}

export interface AskContext {
  /** Порядок ответа смыслового поиска сервера; `null`, если он не нужен или не ответил. */
  meaning: string[] | null;
  /** Сколько листов в вестнике (в индексе только первая страница). */
  gazetteTotal: number | null;
  /** Тексты сайта по ключу: ответы про заказ берут слова страницы «Как приобрести». */
  text: (key: string) => string;
  /** Где человек сейчас: от этого зависит ответ на «о чём эта страница». */
  page?: AskPage;
}

export function answer(index: AskIndex, query: string, lang: Lang, ctx: AskContext): AskReply {
  const parsed = parseAsk(query);
  switch (parsed.intent) {
    case 'summary':
      return summaryReply(index, lang, ctx.gazetteTotal);
    case 'start':
      return startReply(index, lang);
    case 'new':
      return newReply(index, lang);
    case 'tale':
      return parsed.topic
        ? searchReply(index, lang, parsed.topic, null, 'tales', 'tale')
        : taleReply(index, lang, parsed.minutes);
    case 'play':
      return parsed.topic
        ? searchReply(index, lang, parsed.topic, null, 'cards', 'play')
        : playReply(index, lang);
    case 'go':
      return goReply(index, lang, parsed.go as GoSpec);
    case 'page':
      return pageReply(index, lang, ctx.page, ctx.text);
    case 'about':
      return aboutReply(index, lang, parsed.about as AboutKey, !!parsed.price, ctx.text);
    case 'count':
      return countReply(index, lang, parsed.facts ?? [], ctx.gazetteTotal);
    case 'search':
      return searchReply(index, lang, parsed.topic, ctx.meaning, 'all', 'search');
    default:
      return noneReply(lang, '', query);
  }
}
