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
import { matchReply, matchStart, wantsMatch } from '$lib/site-ask-match';
import { leafCopy } from '$lib/gazette';
import { readingMinutes } from '$lib/tales';
import { cardRace, cardTitle, searchSite } from '$lib/site-search';
import type { AskIndex } from '$lib/site-index';
import type { AskHandler, AskRouteReply, BattleCard, FigurineListItem } from '$lib/types/api';
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
  'resum', 'en pocas palabras', 'qué es este sitio', 'que es este sitio', 'qué hay aquí', 'que hay aqui', 'de qué va', 'de que va', 'sobre el sitio',
  'zusammenfass', 'überblick', 'ueberblick', 'kurz:', 'was ist das für', 'was ist das fur', 'was gibt es hier', 'worum geht', 'über die seite',
  'riassum', 'in breve', 'panoramica', 'che sito è', 'che sito e', 'cosa c’è qui', "cosa c'è qui", 'di cosa parla',
];
const START_PHRASES = [
  'с чего начать', 'с чего начинать', 'что посмотреть', 'посоветуй', 'порекомендуй', 'что выбрать', 'что выбрать',
  'з чого почати', 'що подивитися', 'порадь', 'порекомендуй',
  'where to start', 'where do i start', 'what should i', 'recommend', 'suggest',
  'por dónde empiezo', 'por donde empiezo', 'por dónde empezar', 'qué me recomiendas', 'que me recomiendas', 'qué ver', 'recomienda', 'sugiere',
  'womit fange', 'wo fange ich an', 'wo anfangen', 'was empfiehlst', 'empfiehl', 'was soll ich',
  'da dove comincio', 'da dove inizio', 'da dove cominciare', 'cosa mi consigli', 'consiglia', 'cosa guardare', 'suggerisci',
];
const NEW_STEMS = ['нов', 'свеж', 'последн', 'останн', 'new', 'latest', 'recent', 'fresh', 'nuev', 'novedad', 'último', 'ultimo', 'neu', 'neuigkeit', 'nuov', 'novità', 'novita', 'ultim'];
const TALE_STEMS = ['почита', 'почит', 'байк', 'небыл', 'небил', 'рассказ', 'сказк', 'казк', 'истори', 'історі', 'оповід', 'tale', 'story', 'stories', 'read',
  'cuento', 'leer', 'lectura', 'historia', 'geschicht', 'lesen', 'lektüre', 'raccont', 'leggere', 'lettura', 'storia', 'storie'];
const PLAY_STEMS = ['игр', 'сыгра', 'играт', 'этюд', 'бой', 'боя', 'битв', 'гра', 'зіграт', 'грати', 'етюд', 'бій', 'game', 'play', 'etude', 'study', 'battle', 'карт', 'card',
  'jueg', 'jugar', 'étude', 'combate', 'batalla', 'carta', 'spiel', 'etüde', 'kampf', 'karte', 'gioc', 'battagl'];


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
  /** Приветствие или «спасибо» (`chat`). */
  chat?: 'hello' | 'thanks';
}

const ABOUT_PHRASES: [AboutKey, string[]][] = [
  ['author', ['кто автор', 'кто делает', 'кто делал', 'кто создает', 'кто создал', 'кто мастер', 'кто вы', 'кто ты', 'об авторе', 'про автора',
    'who makes', 'who made', 'who is the author', 'who are you', 'about the author',
    'хто автор', 'хто робить', 'хто ви', 'хто ти', 'про авторку',
    'quién es la autora', 'quien es la autora', 'sobre la autora', 'quién hace las', 'quien hace las', 'quién eres', 'quien eres',
    'wer ist die autorin', 'über die autorin', 'wer macht', 'wer bist du',
    'chi è l’autrice', "chi è l'autrice", 'chi e l’autrice', 'sull’autrice', 'chi fa le', 'chi sei']],
  ['order', ['как заказать', 'как купить', 'как приобрести', 'как получить работу', 'как запросить', 'заказать работу', 'купить работу', 'хочу купить', 'хочу заказать',
    'доставк', 'оплат', 'депозит', 'сколько стоит', 'цена', 'цены', 'стоимость',
    'how to order', 'how to buy', 'how can i buy', 'how to acquire', 'commission', 'delivery', 'payment', 'how much', 'price',
    'як замовити', 'як купити', 'як придбати', 'скільки коштує', 'ціна', 'ціни',
    'cómo encargar', 'como encargar', 'encargar', 'cómo comprar', 'como comprar', 'comprar una obra', 'adquirir', 'envío', 'envio', 'pago', 'cuánto cuesta', 'cuanto cuesta', 'precio',
    'wie bestelle', 'bestellen', 'kaufen', 'erwerben', 'versand', 'zahlung', 'was kostet', 'wie viel kostet', 'preis',
    'come ordinare', 'ordinare', 'comprare', 'acquistare', 'spedizione', 'pagamento', 'quanto costa', 'prezzo']],
  ['contact', ['как связаться', 'как написать', 'написать автору', 'связаться с', 'контакт',
    'how to contact', 'contact the author', 'get in touch', 'як зв’язатися', 'як звязатися', 'як написати', 'зв’язатися', 'зв’язок',
    'cómo contactar', 'como contactar', 'contactar', 'escribir a la autora', 'wie erreiche', 'kontakt', 'der autorin schreiben',
    'come contattare', 'contattare', 'scrivere all’autrice']],
  ['made', ['из чего', 'из какого материала', 'материал', 'как сделан', 'как делаются', 'как лепит', 'какого размера', 'какой размер', 'размер',
    'made of', 'what material', 'how are they made', 'how is it made', 'techniq', 'what size', 'how big',
    'з чого', 'з якого матеріалу', 'як зроблен', 'матеріал', 'якого розміру', 'розмір',
    'de qué están hechas', 'de que estan hechas', 'hechas de', 'hecho de', 'material', 'qué tamaño', 'que tamaño', 'tamaño',
    'woraus', 'wie groß', 'wie gross', 'größe', 'groesse', 'wie gemacht',
    'di che cosa sono', 'di cosa sono fatte', 'fatte di', 'materiale', 'dimensioni', 'quanto è grande', 'come sono fatte']],
  ['rules', ['как играть', 'правила', 'что такое этюд', 'что такое битва', 'что такое игра', 'как работает игра', 'how to play', 'rules', 'what is a study', 'what is a battle', 'how does the game',
    'як грати', 'що таке етюд', 'що таке битва', 'що таке гра',
    'cómo se juega', 'como se juega', 'cómo jugar', 'como jugar', 'reglas', 'qué es un étude', 'que es un etude', 'qué es una batalla', 'que es una batalla',
    'wie spielt man', 'wie spiele ich', 'regeln', 'was ist eine etüde', 'was ist eine schlacht',
    'come si gioca', 'come giocare', 'regole', 'cos’è un étude', "cos'è un étude", 'cos’è una battaglia', "cos'è una battaglia"]],
  ['tale', ['что такое небылиц', 'что такое байк', 'what is a tall tale', 'what are tall tales', 'що таке небилиц',
    'qué es un cuento', 'que es un cuento', 'qué son los cuentos', 'was ist eine geschichte', 'was sind die geschichten', 'cos’è un racconto', "cos'è un racconto", 'cosa sono i racconti']],
  ['gazette', ['что такое вестник', 'what is the gazette', 'що таке вісник', 'qué es la gaceta', 'que es la gaceta', 'was ist die zeitung', 'cos’è la gazzetta', "cos'è la gazzetta"]],
];
const PRICE_WORDS = ['сколько стоит', 'цена', 'цены', 'стоимость', 'how much', 'price', 'скільки коштує', 'ціна', 'ціни',
  'cuánto cuesta', 'cuanto cuesta', 'precio', 'was kostet', 'wie viel kostet', 'preis', 'quanto costa', 'prezzo'];

const COUNT_TRIGGERS = ['сколько', 'скільки', 'how many', 'количество', 'кількість', 'cuánt', 'cuant', 'wie viele', 'wie viel', 'anzahl', 'quant'];
const FACT_STEMS: [FactKey, string[]][] = [
  ['tales', ['небыл', 'небил', 'tale', 'байк', 'истори', 'історі', 'cuento', 'geschicht', 'raccont']],
  ['leaves', ['вестник', 'вісник', 'gazette', 'лист', 'gaceta', 'hoja', 'zeitung', 'blätt', 'blatt', 'gazzett', 'fogli']],
  ['cards', ['карт', 'card', 'carta', 'carte', 'karte']],
  ['etudes', ['этюд', 'етюд', 'битв', 'stud', 'etude', 'étude', 'etüde', 'battle', 'batalla', 'schlacht', 'battagl']],
  ['races', ['расы', 'расам', 'расах', 'раси', 'race', 'raza', 'volk', 'völker', 'voelker', 'razz']],
  ['works', ['работ', 'робіт', 'робот', 'work', 'figurin', 'кукл', 'фигур', 'ляльк', 'фігур', 'obra', 'figura', 'muñec', 'werk', 'figur', 'puppe', 'opera', 'opere', 'statuin', 'bambol']],
];
const IN_PROGRESS = ['в работе', 'у роботі', 'in progress', 'в процессе', 'у процесі', 'en proceso', 'in arbeit', 'in lavorazione'];
const EARLIEST = ['самая ранн', 'самая стар', 'самая перв', 'найраніш', 'найстарш', 'перша робота', 'earliest', 'oldest', 'first work', 'с какого года', 'з якого року', 'since when',
  'más antigua', 'mas antigua', 'primera obra', 'desde cuándo', 'desde cuando', 'früheste', 'frueheste', 'älteste', 'erstes werk', 'seit wann', 'più antica', 'piu antica', 'prima opera', 'da quando'];


// Приветствие и «спасибо» разбирают правила, а не каталог: фраза в одно слово
// не несёт смысла, по которому её можно сравнить с записями.
const GREETINGS = ['привет', 'здравствуй', 'здравствуйте', 'добрый день', 'добрый вечер', 'доброе утро', 'приветствую', 'привіт', 'вітаю', 'добрий день', 'добрий вечір', 'доброго дня',
  'hello', 'hi', 'hey', 'good morning', 'good evening', 'good afternoon', 'hola', 'buenos días', 'buenas tardes', 'hallo', 'guten tag', 'guten morgen', 'servus', 'ciao', 'buongiorno', 'buonasera'];
const THANKS = ['спасибо', 'благодарю', 'дякую', 'дякуємо', 'thanks', 'thank you', 'thx', 'gracias', 'danke', 'grazie', 'merci'];
/** Что ещё может стоять в такой фразе, кроме самого приветствия или «спасибо». */
const CHAT_REST = new Set(['большое', 'вам', 'тебе', 'вас', 'за', 'всё', 'все', 'ответ', 'понятно', 'ясно', 'хорошо', 'отлично', 'всем', 'там', 'дуже', 'велике', 'вам', 'тобі', 'дякую', 'there', 'you', 'a', 'lot', 'very', 'much', 'so', 'all', 'ok', 'okay', 'everyone', 'muchas', 'sehr', 'schön', 'mille', 'molte', 'vielen', 'tante', 'todos', 'allen', 'tutti', 'und', 'e', 'y', 'bien', 'gut']);

/** Это приветствие или «спасибо» и больше ничего. */
export function smalltalkOf(q: string): 'hello' | 'thanks' | null {
  let text = ` ${q.replace(/[^\p{L}\p{N}\s]/gu, ' ').replace(/\s+/g, ' ').trim()} `;
  if (text.trim() === '') return null;
  let kind: 'hello' | 'thanks' | null = null;
  for (const [list, which] of [[GREETINGS, 'hello'], [THANKS, 'thanks']] as const) {
    for (const w of list) {
      if (text.includes(` ${w} `)) {
        kind ??= which;
        text = text.replace(` ${w} `, ' ');
      }
    }
  }
  if (!kind) return null;
  const rest = text.trim().split(' ').filter(Boolean);
  return rest.length <= 3 && rest.every((t) => CHAT_REST.has(t)) ? kind : null;
}

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
  const chat = smalltalkOf(q);
  if (chat) return { intent: 'chat', topic: '', minutes, chat };
  if (wantsPage(query)) return { intent: 'page', topic: '', minutes };
  if (has(SUMMARY_PHRASES)) return { intent: 'summary', topic: '', minutes };
  if (wantsMatch(query)) return { intent: 'match', topic: '', minutes };
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

// ── Что в поле: слово для поиска, вопрос или команда ──────────────────────

/** Первое слово фразы, с которого начинается вопрос (шесть языков). Основа без
 *  окончания: «какие», «какой» и «как» ловятся одним началом. */
const QUESTION_OPENERS = [
  'как', 'что ', 'что?', 'где', 'когда', 'сколько', 'почему', 'зачем', 'какой', 'какая', 'какие', 'какое', 'можно', 'есть ли', 'чем ', 'кто', 'куда', 'откуда', 'на что',
  'як', 'що ', 'де ', 'коли', 'скільки', 'чому', 'навіщо', 'який', 'яка', 'які', 'чи ', 'хто', 'куди', 'звідки',
  'how', 'what', 'where', 'when', 'why', 'who', 'which', 'can ', 'could', 'do ', 'does', 'is ', 'are ', 'will ', 'should',
  'cómo', 'como ', 'qué', 'dónde', 'donde ', 'cuándo', 'cuando ', 'cuánto', 'cuanto', 'cuántos', 'por qué', 'quién', 'quien ', 'cuál', 'cual ', 'puedo', 'hay ',
  'wie', 'was ', 'wo ', 'wann', 'warum', 'wer ', 'welche', 'kann', 'gibt', 'womit', 'woher',
  'come ', 'cosa ', 'dove', 'quando', 'perché', 'perche ', 'chi ', 'quale', 'quanto', 'posso', 'c’è', "c'è",
];

/** Что человек, судя по строке, делает: ищет вещь, спрашивает или командует. */
export type AskShape = 'search' | 'question' | 'command';

/** Решение «это вопрос» принимается локально и до сервера. Ответ каталога верен
 *  не всегда (по замеру — примерно в пяти из шести случаев), и если бы он
 *  выскакивал на каждое слово, ошибка стояла бы на виду. Поэтому одно слово
 *  вопросом не считается никогда, а фраза без вопросительного слова — только
 *  когда правила узнали в ней точный вид («как играть», «о чём страница»). */
export function askShape(query: string): AskShape {
  const q = normalizeQuery(query);
  if (!q) return 'search';
  const parsed = parseAsk(query);
  if (parsed.intent === 'go' || parsed.intent === 'chat') return 'command';
  const words = tokensOf(q).length;
  const mark = query.includes('?');
  if (words < 2 && !mark) return 'search';
  const opener = QUESTION_OPENERS.some((o) => (q + ' ').startsWith(o));
  if (mark || opener) return 'question';
  // Виды, у которых точные приметы в самих фразах.
  if (['about', 'start', 'page', 'match', 'summary'].includes(parsed.intent)) return 'question';
  return 'search';
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
    say(lang, 'Это витрина авторских готических миниатюр, а не магазин. ', 'Це вітрина авторських готичних мініатюр, а не магазин. ', 'This is a showcase of original gothic miniatures, not a shop. ', 'Es un escaparate de miniaturas góticas de autora, no una tienda. ', 'Dies ist eine Vitrine eigener gotischer Miniaturen, kein Laden. ', 'È una vetrina di miniature gotiche d’autrice, non un negozio. ') +
    (works.length
      ? say(lang, `В собрании ${n_works(lang, works.length)}`, `У зібранні ${n_works(lang, works.length)}`, `The collection holds ${n_works(lang, works.length)}`, `La colección reúne ${n_works(lang, works.length)}`, `Die Sammlung umfasst ${n_works(lang, works.length)}`, `La raccolta conta ${n_works(lang, works.length)}`) +
        (inProgress
          ? say(lang, `, из них ${inProgress} в работе`, `, з них ${inProgress} у роботі`, `, ${inProgress} of them still in progress`, `, ${inProgress} de ellas todavía en proceso`, `, davon ${inProgress} noch in Arbeit`, `, di cui ${inProgress} ancora in lavorazione`)
          : '') +
        (earliest
          ? say(lang, `; самая ранняя — ${earliest} года.`, `; найраніша — ${earliest} року.`, `; the earliest is from ${earliest}.`, `; la más antigua es de ${earliest}.`, `; das früheste stammt von ${earliest}.`, `; la più antica è del ${earliest}.`)
          : '.')
      : '');

  const beside: string[] = [];
  if (index.tales.length) beside.push(n_tales(lang, index.tales.length));
  if (leaves) beside.push(n_leaves(lang, leaves));
  if (cards.length) beside.push(say(lang, `${n_cards(lang, cards.length)} в игре`, `${n_cards(lang, cards.length)} у грі`, `${n_cards(lang, cards.length)} in the game`, `${n_cards(lang, cards.length)} en el juego`, `${n_cards(lang, cards.length)} im Spiel`, `${n_cards(lang, cards.length)} nel gioco`));
  if (etudes.length) {
    beside.push(
      say(lang, `${n_etudes(lang, etudes.length)} — партии против компьютера`, `${n_etudes(lang, etudes.length)} — партії проти комп’ютера`, `${n_etudes(lang, etudes.length)} — games against the computer`, `${n_etudes(lang, etudes.length)}: partidas contra el ordenador`, `${n_etudes(lang, etudes.length)} – Partien gegen den Computer`, `${n_etudes(lang, etudes.length)}: partite contro il computer`),
    );
  }
  const second = beside.length
    ? say(lang, 'Рядом с работами: ', 'Поряд із роботами: ', 'Alongside the works: ', 'Junto a las obras: ', 'Neben den Werken: ', 'Accanto alle opere: ') + joinList(lang, beside) + '.'
    : '';

  const races = new Map<string, number>();
  for (const c of cards) {
    const r = cardRace(c, lang);
    if (r) races.set(r, (races.get(r) ?? 0) + 1);
  }
  const topRaces = [...races.entries()].filter(([, n]) => n >= 2).sort((a, b) => b[1] - a[1]).slice(0, 3).map(([r]) => r);
  const third = topRaces.length >= 2
    ? say(lang, `Чаще всего среди карт: ${topRaces.join(', ')}.`, `Найчастіше серед карт: ${topRaces.join(', ')}.`, `Most common among the cards: ${topRaces.join(', ')}.`, `Lo más frecuente entre las cartas: ${topRaces.join(', ')}.`, `Am häufigsten unter den Karten: ${topRaces.join(', ')}.`, `Le più frequenti tra le carte: ${topRaces.join(', ')}.`)
    : '';

  const latestWork = newest(works, (w) => w.createdAt)[0];
  const latestTale = talesNewest(index)[0];
  const lastBits: string[] = [];
  if (latestWork) lastBits.push(say(lang, `работа ${quote(workName(latestWork))}`, `робота ${quote(workName(latestWork))}`, `the work ${quote(workName(latestWork))}`, `la obra ${quote(workName(latestWork))}`, `das Werk ${quote(workName(latestWork))}`, `l’opera ${quote(workName(latestWork))}`));
  if (latestTale) lastBits.push(say(lang, `небылица ${quote(leafTitle(latestTale, lang))}`, `небилиця ${quote(leafTitle(latestTale, lang))}`, `the tall tale ${quote(leafTitle(latestTale, lang))}`, `el cuento ${quote(leafTitle(latestTale, lang))}`, `die Geschichte ${quote(leafTitle(latestTale, lang))}`, `il racconto ${quote(leafTitle(latestTale, lang))}`));
  const fourth = lastBits.length ? say(lang, 'Последнее: ', 'Останнє: ', 'Latest: ', 'Lo último: ', 'Zuletzt: ', 'Le ultime novità: ') + joinList(lang, lastBits) + '.' : '';

  const links: AskLink[] = [];
  if (latestWork) links.push(workLink(latestWork, lang, say(lang, 'новая работа', 'нова робота', 'newest work', 'obra más reciente', 'neuestes Werk', 'opera più recente')));
  if (latestTale) links.push(taleLink(latestTale, lang));
  links.push({ kind: 'section', label: say(lang, 'Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'), hint: String(works.length), href: '/figurines' });
  if (index.tales.length) links.push({ kind: 'section', label: say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), hint: String(index.tales.length), href: '/tales' });
  if (leaves) links.push({ kind: 'section', label: say(lang, 'Вестник', 'Вісник', 'Gazette', 'Gaceta', 'Zeitung', 'Gazzetta'), hint: String(leaves), href: '/gazette' });
  const firstEtude = etudes[0];
  links.push(
    firstEtude
      ? { kind: 'section', label: say(lang, 'Игра', 'Гра', 'The game', 'El juego', 'Das Spiel', 'Il gioco'), hint: etudeTitle(firstEtude, lang), href: '/battles/etude' }
      : { kind: 'section', label: say(lang, 'Игра', 'Гра', 'The game', 'El juego', 'Das Spiel', 'Il gioco'), href: '/battles' },
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
      paragraphs: [say(lang, 'Пока на сайте нечего посоветовать: попробуйте открыть архив работ.', 'Поки на сайті нічого порадити: спробуйте відкрити архів робіт.', 'There is nothing to suggest yet: try the archive of works.', 'Todavía no hay nada que sugerir: mira el archivo de obras.', 'Noch gibt es nichts vorzuschlagen: Sieh im Archiv der Werke nach.', 'Non c’è ancora niente da suggerire: prova l’archivio delle opere.')],
      links: [{ kind: 'section', label: say(lang, 'Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'), href: '/figurines' }],
    };
  }

  const links: AskLink[] = [];
  if (work) links.push(workLink(work, lang));
  if (tale) links.push(taleLink(tale, lang));
  if (card) links.push(cardLink(card, lang));

  const parts: string[] = [];
  if (tale) {
    const min = readingMinutes(leafCopy(tale, lang).body);
    parts.push(say(lang, `небылица ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `небилиця ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `the tall tale ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `el cuento ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `die Geschichte ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`, `il racconto ${quote(leafTitle(tale, lang))} (${short_min(lang, min)})`));
  }
  if (card) parts.push(say(lang, 'карта в игре', 'карта у грі', 'a card in the game', 'una carta en el juego', 'eine Karte im Spiel', 'una carta nel gioco'));

  const lead = work
    ? say(lang, `Начните с работы ${quote(workName(work))}`, `Почніть із роботи ${quote(workName(work))}`, `Start with the work ${quote(workName(work))}`, `Empieza por la obra ${quote(workName(work))}`, `Fang mit dem Werk ${quote(workName(work))} an`, `Comincia dall’opera ${quote(workName(work))}`)
    : say(lang, 'Начните с небылицы', 'Почніть із небилиці', 'Start with a tall tale', 'Empieza por un cuento', 'Fang mit einer Geschichte an', 'Comincia da un racconto');
  const body = parts.length
    ? say(lang, `: о ней есть ${joinList(lang, parts)}.`, `: про неї є ${joinList(lang, parts)}.`, `: it comes with ${joinList(lang, parts)}.`, `: la acompañan ${joinList(lang, parts)}.`, `: Dazu gehören ${joinList(lang, parts)}.`, `: la accompagnano ${joinList(lang, parts)}.`)
    : '.';
  const order = say(
    lang,
    'Порядок простой: посмотреть, прочитать, сыграть.',
    'Порядок простий: подивитися, прочитати, зіграти.',
    'The order is simple: look, read, play.',
    'El orden es sencillo: mirar, leer, jugar.',
    'Der Ablauf ist einfach: ansehen, lesen, spielen.',
    'L’ordine è semplice: guardare, leggere, giocare.',
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
    sentences.push(say(lang, `Недавно в собрании появились: ${joinList(lang, names)}.`, `Нещодавно в зібранні з’явилися: ${joinList(lang, names)}.`, `Recently added to the collection: ${joinList(lang, names)}.`, `Añadidas hace poco a la colección: ${joinList(lang, names)}.`, `Neu in der Sammlung: ${joinList(lang, names)}.`, `Aggiunte da poco alla raccolta: ${joinList(lang, names)}.`));
  }
  const reading: string[] = [];
  if (tale) reading.push(say(lang, `небылица ${quote(leafTitle(tale, lang))}`, `небилиця ${quote(leafTitle(tale, lang))}`, `the tall tale ${quote(leafTitle(tale, lang))}`, `el cuento ${quote(leafTitle(tale, lang))}`, `die Geschichte ${quote(leafTitle(tale, lang))}`, `il racconto ${quote(leafTitle(tale, lang))}`));
  if (leaf) reading.push(say(lang, `в вестнике — ${quote(leafTitle(leaf, lang))}`, `у віснику — ${quote(leafTitle(leaf, lang))}`, `in the gazette ${quote(leafTitle(leaf, lang))}`, `en la Gaceta ${quote(leafTitle(leaf, lang))}`, `in der Zeitung ${quote(leafTitle(leaf, lang))}`, `nella Gazzetta ${quote(leafTitle(leaf, lang))}`));
  if (reading.length) {
    sentences.push(say(lang, `Из чтения новее всего: ${joinList(lang, reading)}.`, `З читання найновіше: ${joinList(lang, reading)}.`, `Newest reading: ${joinList(lang, reading)}.`, `Lo más reciente para leer: ${joinList(lang, reading)}.`, `Neu zum Lesen: ${joinList(lang, reading)}.`, `Le letture più recenti: ${joinList(lang, reading)}.`));
  }
  if (!sentences.length) {
    sentences.push(say(lang, 'Пока новых записей нет.', 'Поки нових записів немає.', 'Nothing new yet.', 'Todavía no hay nada nuevo.', 'Noch nichts Neues.', 'Ancora niente di nuovo.'));
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
      paragraphs: [say(lang, 'Небылиц для чтения пока нет.', 'Небилиць для читання поки немає.', 'There are no tall tales to read yet.', 'Todavía no hay cuentos para leer.', 'Noch gibt es keine Geschichten zu lesen.', 'Non ci sono ancora racconti da leggere.')],
      links: [{ kind: 'section', label: say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), href: '/tales' }],
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
      `Para ${n_minutes(lang, minutes)}: ${joinList(lang, shown.map((x) => quote(leafTitle(x.l, lang))))}.`,
      `Für ${n_minutes(lang, minutes)} passt: ${joinList(lang, shown.map((x) => quote(leafTitle(x.l, lang))))}.`,
      `Per ${n_minutes(lang, minutes)} va bene: ${joinList(lang, shown.map((x) => quote(leafTitle(x.l, lang))))}.`,
    );
  } else if (minutes) {
    text = say(
      lang,
      `Короче ${n_minutes(lang, minutes)} небылиц нет. Самая короткая — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} чтения.`,
      `Коротших за ${n_minutes(lang, minutes)} небилиць немає. Найкоротша — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} читання.`,
      `There is no tall tale shorter than ${n_minutes(lang, minutes)}. The shortest is ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} to read.`,
      `No hay cuentos de menos de ${n_minutes(lang, minutes)}. El más corto es ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} de lectura.`,
      `Keine Geschichte ist kürzer als ${n_minutes(lang, minutes)}. Die kürzeste ist ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} Lesezeit.`,
      `Non ci sono racconti più brevi di ${n_minutes(lang, minutes)}. Il più breve è ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} di lettura.`,
    );
  } else {
    text = say(
      lang,
      `На полке ${n_tales(lang, index.tales.length)}. Самая короткая — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} чтения.`,
      `На полиці ${n_tales(lang, index.tales.length)}. Найкоротша — ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} читання.`,
      `The shelf holds ${n_tales(lang, index.tales.length)}. The shortest is ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} to read.`,
      `En el estante hay ${n_tales(lang, index.tales.length)}. El más corto es ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} de lectura.`,
      `Im Regal stehen ${n_tales(lang, index.tales.length)}. Die kürzeste ist ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} Lesezeit.`,
      `Sullo scaffale ci sono ${n_tales(lang, index.tales.length)}. Il più breve è ${quote(leafTitle(shown[0].l, lang))}, ${n_minutes(lang, shown[0].min)} di lettura.`,
    );
  }
  const links = shown.map((x) => taleLink(x.l, lang));
  links.push({ kind: 'section', label: say(lang, 'Все небылицы', 'Усі небилиці', 'All tall tales', 'Todos los cuentos', 'Alle Geschichten', 'Tutti i racconti'), hint: String(index.tales.length), href: '/tales' });
  return { intent: 'tale', paragraphs: [text], links };
}

function playReply(index: AskIndex, lang: Lang): AskReply {
  const cards = heroCards(index);
  const etudes = etudesInOrder(index);
  const first = etudes[0];

  const paragraphs = [
    say(
      lang,
      `В игре ${n_cards(lang, cards.length)}. Битва — это партия против компьютера: вы расставляете карты и играете.`,
      `У грі ${n_cards(lang, cards.length)}. Битва — це партія проти комп’ютера: ви розставляєте карти й граєте.`,
      `The game has ${n_cards(lang, cards.length)}. A battle is a game against the computer: you place your cards and play.`,
      `El juego tiene ${n_cards(lang, cards.length)}. Una batalla es una partida contra el ordenador: colocas tus cartas y juegas.`,
      `Im Spiel gibt es ${n_cards(lang, cards.length)}. Eine Schlacht ist eine Partie gegen den Computer: Du stellst deine Karten auf und spielst.`,
      `Il gioco ha ${n_cards(lang, cards.length)}. Una battaglia è una partita contro il computer: disponi le tue carte e giochi.`,
    ),
  ];
  if (first) {
    paragraphs.push(
      say(
        lang,
        `Сейчас ${n_etudes(lang, etudes.length)}; первый — ${quote(etudeTitle(first, lang))}. Карты можно смотреть и без игры — на полке.`,
        `Зараз ${n_etudes(lang, etudes.length)}; перший — ${quote(etudeTitle(first, lang))}. Карти можна дивитися й без гри — на полиці.`,
        `There ${etudes.length === 1 ? 'is' : 'are'} ${n_etudes(lang, etudes.length)} now; the first is ${quote(etudeTitle(first, lang))}. You can also just look at the cards on the shelf.`,
        `Ahora hay ${n_etudes(lang, etudes.length)}; el primero es ${quote(etudeTitle(first, lang))}. Las cartas también se pueden ver sin jugar, en el estante.`,
        `Derzeit gibt es ${n_etudes(lang, etudes.length)}; die erste ist ${quote(etudeTitle(first, lang))}. Die Karten kannst du auch ohne Spiel im Regal ansehen.`,
        `Ora ci sono ${n_etudes(lang, etudes.length)}; il primo è ${quote(etudeTitle(first, lang))}. Le carte si possono guardare anche senza giocare, sullo scaffale.`,
      ),
    );
  }
  const links: AskLink[] = [];
  if (first) links.push(etudeLink(first, lang));
  links.push({ kind: 'section', label: say(lang, 'Полка карт', 'Полиця карт', 'Card shelf', 'Estante de cartas', 'Kartenregal', 'Scaffale delle carte'), hint: String(cards.length), href: '/battles' });
  if (etudes.length > 1) links.push({ kind: 'section', label: say(lang, 'Все битвы', 'Усі битви', 'All battles', 'Todas las batallas', 'Alle Schlachten', 'Tutte le battaglie'), hint: String(etudes.length), href: '/battles/etude' });
  return { intent: 'play', paragraphs, links };
}

/** Вопросы, которые сайт умеет: из них «не умею» предлагает ближайшие к сказанному. */
function examples(lang: Lang): string[] {
  return [
    say(lang, 'Коротко: что это за сайт?', 'Коротко: що це за сайт?', 'Briefly, what is this site?', 'En pocas palabras, ¿qué es este sitio?', 'Kurz: Was ist das für eine Seite?', 'In breve, che sito è questo?'),
    say(lang, 'Сколько работ на сайте?', 'Скільки робіт на сайті?', 'How many works are on the site?', '¿Cuántas obras hay en el sitio?', 'Wie viele Werke gibt es auf der Seite?', 'Quante opere ci sono sul sito?'),
    say(lang, 'Что нового?', 'Що нового?', 'What is new?', '¿Qué hay de nuevo?', 'Was gibt es Neues?', 'Che novità ci sono?'),
    say(lang, 'Что почитать за 5 минут?', 'Що почитати за 5 хвилин?', 'A tale to read in 5 minutes', 'Un cuento para leer en 5 minutos', 'Eine Geschichte für 5 Minuten', 'Un racconto da leggere in 5 minuti'),
    say(lang, 'С чего начать?', 'З чого почати?', 'Where to start?', '¿Por dónde empiezo?', 'Womit fange ich an?', 'Da dove comincio?'),
    say(lang, 'Во что можно сыграть?', 'У що можна зіграти?', 'What can I play?', '¿A qué puedo jugar?', 'Was kann ich spielen?', 'A cosa posso giocare?'),
    say(lang, 'Как играть?', 'Як грати?', 'How to play?', '¿Cómo se juega?', 'Wie spielt man?', 'Come si gioca?'),
    say(lang, 'Как заказать работу?', 'Як замовити роботу?', 'How to order a work?', '¿Cómo encargar una obra?', 'Wie bestelle ich ein Werk?', 'Come ordinare un’opera?'),
    say(lang, 'Из чего сделаны фигурки?', 'З чого зроблені фігурки?', 'What are the figurines made of?', '¿De qué están hechas las figuras?', 'Woraus sind die Figuren gemacht?', 'Di che cosa sono fatte le statuine?'),
    say(lang, 'Кто автор?', 'Хто автор?', 'Who is the author?', '¿Quién es la autora?', 'Wer ist die Autorin?', 'Chi è l’autrice?'),
    say(lang, 'Как связаться с автором?', 'Як зв’язатися з автором?', 'How to contact the author?', '¿Cómo contactar con la autora?', 'Wie erreiche ich die Autorin?', 'Come contattare l’autrice?'),
    say(lang, 'Какая самая ранняя работа?', 'Яка найраніша робота?', 'Which is the earliest work?', '¿Cuál es la obra más antigua?', 'Welches ist das früheste Werk?', 'Qual è l’opera più antica?'),
    say(lang, 'Покажи ведьм', 'Покажи відьом', 'Show me a witch', 'Enséñame una bruja', 'Zeig mir eine Hexe', 'Mostrami una strega'),
    say(lang, 'Сколько рас в игре?', 'Скільки рас у грі?', 'How many races are in the game?', '¿Cuántas razas hay en el juego?', 'Wie viele Völker gibt es im Spiel?', 'Quante razze ci sono nel gioco?'),
  ];
}

/** Тот же вопрос, что и спрошенный: без регистра и знаков препинания. Предлагать
 *  человеку вопрос, который он только что задал, значит отвечать ему его же словами. */
function sameQuestion(asked: string): (other: string) => boolean {
  const fold = (text: string) => tokensOf(normalizeQuery(text)).join(' ');
  const own = fold(asked);
  return (other) => !!own && fold(other) === own;
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
  const again = sameQuestion(query);
  const ranked = examples(lang)
    .map((q, i) => {
      const own = stems(q);
      let hits = 0;
      for (const w of own) if (asked.has(w)) hits += 1;
      return { q, i, hits };
    })
    .filter((x) => !again(x.q))
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
          `No he encontrado nada para ${quote(topic)}. Prueba con un personaje o un material («bruja», «gnomo», «lana») o pregunta así:`,
          `Zu ${quote(topic)} wurde nichts gefunden. Nenn eine Figur oder ein Material („Hexe“, „Gnom“, „Wolle“) oder frag so:`,
          `Per ${quote(topic)} non ho trovato nulla. Prova con un personaggio o un materiale («strega», «gnomo», «lana») oppure chiedi così:`,
        ),
      ]
    : [
        say(
          lang,
          'Такой вопрос я пока не умею. Я отвечаю по данным самого сайта: рассказываю о нём, считаю работы, небылицы и карты, ищу по темам, подсказываю, что почитать и во что сыграть. Например:',
          'Таке питання я поки не вмію. Я відповідаю за даними самого сайту: розповідаю про нього, рахую роботи, небилиці й карти, шукаю за темами, підказую, що почитати й у що зіграти. Наприклад:',
          'I cannot answer that yet. I answer from the site’s own data: I describe it, count works, tall tales and cards, search by topic, and suggest what to read and play. For example:',
          'Todavía no sé responder a esa pregunta. Respondo con los datos del propio sitio: lo describo, cuento obras, cuentos y cartas, busco por temas y sugiero qué leer y a qué jugar. Por ejemplo:',
          'Diese Frage kann ich noch nicht beantworten. Ich antworte aus den Daten der Seite selbst: Ich beschreibe sie, zähle Werke, Geschichten und Karten, suche nach Themen und schlage vor, was du lesen und spielen kannst. Zum Beispiel:',
          'A questa domanda non so ancora rispondere. Rispondo con i dati del sito stesso: lo descrivo, conto opere, racconti e carte, cerco per temi e suggerisco cosa leggere e a cosa giocare. Per esempio:',
        ),
      ];
  return { intent: 'none', missed: true, paragraphs, links: nearestQuestions(lang, query) };
}

type SearchScopeAsk = 'all' | 'tales' | 'cards';

/** Вопрос, на который нечего ответить, не «запрос»: слова, оставшиеся после «как» и
 *  «что» («выиграть»), не тема, и «по запросу „выиграть“ ничего не нашлось»
 *  отвечало бы на вопрос, которого человек не задавал. */
function asQuestion(reply: AskReply, lang: Lang, query: string): AskReply {
  // «Близко по смыслу» с работами — не пустой ответ, его оставляем.
  const empty = !reply.links.some((l) => l.kind !== 'question');
  return reply.missed && empty && askShape(query) === 'question' ? { ...noneReply(lang, '', query), intent: reply.intent } : reply;
}

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
          `No hay coincidencias exactas para ${quote(topic)}. Lo más cercano en sentido: ${joinList(lang, near.map((f) => quote(workName(f.item))))}. Si no es eso, nombra un personaje o un material.`,
          `Keine genauen Treffer für ${quote(topic)}. Am nächsten im Sinn: ${joinList(lang, near.map((f) => quote(workName(f.item))))}. Wenn das nicht passt, nenn eine Figur oder ein Material.`,
          `Nessuna corrispondenza esatta per ${quote(topic)}. Il più vicino nel senso: ${joinList(lang, near.map((f) => quote(workName(f.item))))}. Se non è questo, nomina un personaggio o un materiale.`,
        ),
      ],
      links: near.map((f) => workLink(f.item, lang, say(lang, 'близко по смыслу', 'близько за змістом', 'close in meaning', 'de sentido parecido', 'ähnlich im Sinn', 'simile nel senso'))),
    };
  }

  const parts: string[] = [];
  if (works.length) parts.push(n_works(lang, works.length));
  if (tales.length) parts.push(n_tales(lang, tales.length));
  if (leaves.length) parts.push(n_leaves(lang, leaves.length));
  if (cards.length) parts.push(n_cards(lang, cards.length));

  const top = works[0]
    ? say(lang, `работа ${quote(workName(works[0].item))}`, `робота ${quote(workName(works[0].item))}`, `the work ${quote(workName(works[0].item))}`, `la obra ${quote(workName(works[0].item))}`, `das Werk ${quote(workName(works[0].item))}`, `l’opera ${quote(workName(works[0].item))}`)
    : tales[0]
      ? say(lang, `небылица ${quote(leafTitle(tales[0].item, lang))}`, `небилиця ${quote(leafTitle(tales[0].item, lang))}`, `the tall tale ${quote(leafTitle(tales[0].item, lang))}`, `el cuento ${quote(leafTitle(tales[0].item, lang))}`, `die Geschichte ${quote(leafTitle(tales[0].item, lang))}`, `il racconto ${quote(leafTitle(tales[0].item, lang))}`)
      : cards[0]
        ? say(lang, `карта ${quote(cardTitle(cards[0].item, lang))}`, `карта ${quote(cardTitle(cards[0].item, lang))}`, `the card ${quote(cardTitle(cards[0].item, lang))}`, `la carta ${quote(cardTitle(cards[0].item, lang))}`, `die Karte ${quote(cardTitle(cards[0].item, lang))}`, `la carta ${quote(cardTitle(cards[0].item, lang))}`)
        : say(lang, `лист вестника ${quote(leafTitle(leaves[0].item, lang))}`, `аркуш вісника ${quote(leafTitle(leaves[0].item, lang))}`, `the gazette leaf ${quote(leafTitle(leaves[0].item, lang))}`, `la hoja de la Gaceta ${quote(leafTitle(leaves[0].item, lang))}`, `das Blatt der Zeitung ${quote(leafTitle(leaves[0].item, lang))}`, `il foglio della Gazzetta ${quote(leafTitle(leaves[0].item, lang))}`);

  const paragraphs = [
    say(lang, `По запросу ${quote(topic)} нашлось: ${joinList(lang, parts)}.`, `За запитом ${quote(topic)} знайшлося: ${joinList(lang, parts)}.`, `For ${quote(topic)} I found ${joinList(lang, parts)}.`, `Para ${quote(topic)} he encontrado ${joinList(lang, parts)}.`, `Zu ${quote(topic)} habe ich ${joinList(lang, parts)} gefunden.`, `Per ${quote(topic)} ho trovato ${joinList(lang, parts)}.`),
    say(lang, `Ближе всего — ${top}.`, `Найближче — ${top}.`, `Closest: ${top}.`, `Lo más cercano: ${top}.`, `Am nächsten: ${top}.`, `Il più vicino: ${top}.`),
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
            'Todas las figuras las hace una sola autora: cada una la modela, la pinta y la viste a mano, por eso no hay dos iguales.',
            'Alle Figuren macht eine einzige Autorin: Jede modelliert, bemalt und kleidet sie von Hand, deshalb gleicht keine der anderen.',
            'Tutte le statuine sono opera di una sola autrice: ognuna è modellata, dipinta e vestita a mano, per questo non ce ne sono due uguali.',
          ),
        ],
        links: [
          sec(say(lang, 'Об авторе', 'Про автора', 'About the author', 'Sobre la autora', 'Über die Autorin', 'L’autrice'), '/author'),
          sec(say(lang, 'Мастерская', 'Майстерня', 'The workshop', 'El taller', 'Die Werkstatt', 'Il laboratorio'), '/workshop'),
        ],
      };
    case 'order': {
      const lead = price
        ? say(
            lang,
            'Цена работы в наличии указана на её странице на Etsy; цену заказной фигурки мы обсуждаем в переписке. ',
            'Ціна роботи в наявності вказана на її сторінці на Etsy; ціну замовної фігурки ми обговорюємо в листуванні. ',
            'The price of a work in stock is on its Etsy page; the price of a commissioned figurine is discussed in a message. ',
            'El precio de una obra disponible está en su página de Etsy; el de una figura por encargo se habla por mensaje. ',
            'Der Preis eines verfügbaren Werks steht auf seiner Etsy-Seite; den Preis einer Auftragsfigur besprechen wir per Nachricht. ',
            'Il prezzo di un’opera disponibile è sulla sua pagina Etsy; quello di una figura su commissione si discute per messaggio. ',
          )
        : '';
      return {
        intent: 'about',
        paragraphs: [
          lead + text('acquireStep1Body'),
          say(lang, 'Порядок такой: ', 'Порядок такий: ', 'The order is: ', 'El orden es: ', 'Der Ablauf: ', 'L’ordine è questo: ') +
            [text('acquireStep1Title'), text('acquireStep2Title'), text('acquireStep3Title'), text('acquireStep4Title')].join(' → ') + '.',
          text('acquireStep4Body'),
        ],
        links: [
          sec(text('acquireTitle'), '/acquire'),
          sec(text('acquireBrowseCta'), '/figurines'),
          sec(text('acquireCommissionCta'), '/commission'),
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
            'Puedes escribir a la autora con el botón «Escribir» de la cabecera o con el formulario de la página principal. La autora lee cada mensaje y responde personalmente, normalmente en un día.',
            'Du erreichst die Autorin über die Schaltfläche „Schreiben“ in der Kopfzeile oder das Formular auf der Startseite. Die Autorin liest jede Nachricht und antwortet persönlich, meist innerhalb eines Tages.',
            'Puoi scrivere all’autrice con il pulsante «Scrivi» nell’intestazione o con il modulo nella pagina principale. L’autrice legge ogni messaggio e risponde di persona, di solito entro un giorno.',
          ),
        ],
        links: [sec(say(lang, 'Об авторе', 'Про автора', 'About the author', 'Sobre la autora', 'Über die Autorin', 'L’autrice'), '/author'), sec(text('acquireTitle'), '/acquire')],
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
              `La base es ${joinList(lang, mats)}. Las figuras se modelan, se pintan y se visten a mano, y cada una existe en un único ejemplar.`,
              `Die Grundlage: ${joinList(lang, mats)}. Die Figuren werden von Hand modelliert, bemalt und gekleidet, und jede gibt es nur ein einziges Mal.`,
              `La base è ${joinList(lang, mats)}. Le statuine sono modellate, dipinte e vestite a mano, e ognuna esiste in un solo esemplare.`,
            )
          : say(
              lang,
              'Фигурки лепят, расписывают и одевают вручную, и каждая существует в единственном экземпляре.',
              'Фігурки ліплять, розписують і одягають вручну, і кожна існує в єдиному екземплярі.',
              'The figurines are sculpted, painted and dressed by hand, and each exists in a single copy.',
              'Las figuras se modelan, se pintan y se visten a mano, y cada una existe en un único ejemplar.',
              'Die Figuren werden von Hand modelliert, bemalt und gekleidet, und jede gibt es nur ein einziges Mal.',
              'Le statuine sono modellate, dipinte e vestite a mano, e ognuna esiste in un solo esemplare.',
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
            `El tamaño es de unos ${range} cm: el exacto figura en la página de cada obra.`,
            `Die Größe beträgt etwa ${range} cm; die genaue steht auf der Seite jedes Werks.`,
            `Le dimensioni sono di circa ${range} cm: quelle esatte sono sulla pagina di ogni opera.`,
          ),
        );
      }
      return {
        intent: 'about',
        paragraphs,
        links: [sec(say(lang, 'Мастерская', 'Майстерня', 'The workshop', 'El taller', 'Die Werkstatt', 'Il laboratorio'), '/workshop'), sec(say(lang, 'Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'), '/figurines')],
      };
    }
    case 'rules':
      return {
        intent: 'about',
        paragraphs: [
          say(
            lang,
            'Битва — партия против компьютера на небольшом поле. Вы расставляете карты и ходите по очереди; выигрывает тот, у кого остались карты, когда у противника их не осталось.',
            'Битва — партія проти комп’ютера на невеликому полі. Ви розставляєте карти й ходите по черзі; виграє той, у кого залишилися карти, коли в противника їх не залишилося.',
            'A battle is a game against the computer on a small field. You place your cards and take turns; whoever still has cards when the opponent has none wins.',
            'Una batalla es una partida contra el ordenador en un campo pequeño. Colocas tus cartas y se juega por turnos; gana quien conserva cartas cuando al rival no le queda ninguna.',
            'Eine Schlacht ist eine Partie gegen den Computer auf einem kleinen Feld. Du stellst deine Karten auf, und ihr zieht abwechselnd; es gewinnt, wer noch Karten hat, wenn der Gegner keine mehr hat.',
            'Una battaglia è una partita contro il computer su un piccolo campo. Disponi le tue carte e si gioca a turno; vince chi ha ancora carte quando l’avversario non ne ha più.',
          ),
          say(
            lang,
            'У каждой битвы могут быть свои правила: они написаны на листе боя перед началом. Первые бои можно играть без входа.',
            'У кожної битви можуть бути свої правила: вони написані на аркуші бою перед початком. Перші бої можна грати без входу.',
            'Each battle may have its own rules: they are written on the battle sheet before it starts. The first battles can be played without signing in.',
            'Cada batalla puede tener sus propias reglas: están escritas en la hoja de combate antes de empezar. Los primeros combates se pueden jugar sin entrar.',
            'Jede Schlacht kann eigene Regeln haben: Sie stehen vor dem Start auf dem Kampfblatt. Die ersten Kämpfe kannst du ohne Anmeldung spielen.',
            'Ogni battaglia può avere regole proprie: sono scritte sul foglio della battaglia prima dell’inizio. Le prime battaglie si possono giocare senza entrare.',
          ),
        ],
        links: [sec(say(lang, 'Битвы', 'Битви', 'Battles', 'Batallas', 'Schlachten', 'Battaglie'), '/battles/etude'), sec(say(lang, 'Полка карт', 'Полиця карт', 'Card shelf', 'Estante de cartas', 'Kartenregal', 'Scaffale delle carte'), '/battles')],
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
            'Un cuento es una historia breve sobre una figura. Muchos tienen su propia obra, y la obra tiene una carta en el juego.',
            'Eine Geschichte ist ein kurzer Text über eine Figur. Viele haben ein eigenes Werk, und das Werk hat eine Karte im Spiel.',
            'Un racconto è una breve storia su una statuina. Molti hanno una propria opera, e l’opera ha una carta nel gioco.',
          ),
        ],
        links: [sec(say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), '/tales', String(index.tales.length || ''))],
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
            'La Gaceta es una serie de hojas breves: lo que ha llegado al taller y noticias sobre miniaturas. Los cuentos también son hojas, pero tienen su propio estante.',
            'Die Zeitung ist eine Folge kurzer Blätter: was in der Werkstatt neu ist und Neuigkeiten rund um Miniaturen. Geschichten sind auch Blätter, haben aber ihr eigenes Regal.',
            'La Gazzetta è una serie di brevi fogli: cosa è arrivato nel laboratorio e notizie sulle miniature. Anche i racconti sono fogli, ma hanno il loro scaffale.',
          ),
        ],
        links: [sec(say(lang, 'Вестник', 'Вісник', 'Gazette', 'Gaceta', 'Zeitung', 'Gazzetta'), '/gazette'), sec(say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), '/tales')],
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
        links.push({ kind: 'section', label: say(lang, 'Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'), hint: String(works.length), href: '/figurines' });
        break;
      case 'inprogress':
        parts.push(
          inProgress
            ? say(lang, `${n_works(lang, inProgress)} ещё в работе`, `${n_works(lang, inProgress)} ще в роботі`, `${n_works(lang, inProgress)} still in progress`, `${n_works(lang, inProgress)} todavía en proceso`, `${n_works(lang, inProgress)} noch in Arbeit`, `${n_works(lang, inProgress)} ancora in lavorazione`)
            : say(lang, 'работ «в работе» сейчас нет', 'робіт «у роботі» зараз немає', 'no works “in progress” right now', 'ahora no hay obras «en proceso»', 'gerade keine Werke „in Arbeit“', 'al momento nessuna opera «in lavorazione»'),
        );
        break;
      case 'tales':
        parts.push(n_tales(lang, index.tales.length));
        links.push({ kind: 'section', label: say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), hint: String(index.tales.length), href: '/tales' });
        break;
      case 'leaves':
        parts.push(n_leaves(lang, leaves));
        links.push({ kind: 'section', label: say(lang, 'Вестник', 'Вісник', 'Gazette', 'Gaceta', 'Zeitung', 'Gazzetta'), hint: String(leaves), href: '/gazette' });
        break;
      case 'cards':
        parts.push(say(lang, `${n_cards(lang, cards.length)} в игре`, `${n_cards(lang, cards.length)} у грі`, `${n_cards(lang, cards.length)} in the game`, `${n_cards(lang, cards.length)} en el juego`, `${n_cards(lang, cards.length)} im Spiel`, `${n_cards(lang, cards.length)} nel gioco`));
        links.push({ kind: 'section', label: say(lang, 'Полка карт', 'Полиця карт', 'Card shelf', 'Estante de cartas', 'Kartenregal', 'Scaffale delle carte'), hint: String(cards.length), href: '/battles' });
        break;
      case 'etudes':
        parts.push(n_etudes(lang, etudes.length));
        links.push({ kind: 'section', label: say(lang, 'Битвы', 'Битви', 'Battles', 'Batallas', 'Schlachten', 'Battaglie'), hint: String(etudes.length), href: '/battles/etude' });
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
              `El juego tiene ${list.length} razas: ${list.map(([r, n]) => `${r} (${n})`).join(', ')}.`,
              `Das Spiel hat ${list.length} Völker: ${list.map(([r, n]) => `${r} (${n})`).join(', ')}.`,
              `Il gioco ha ${list.length} razze: ${list.map(([r, n]) => `${r} (${n})`).join(', ')}.`,
            ),
          );
        } else {
          paragraphs.push(say(lang, 'Расы в игре пока не назначены.', 'Раси в грі поки не призначені.', 'No races are set in the game yet.', 'Todavía no hay razas en el juego.', 'Im Spiel gibt es noch keine Völker.', 'Nel gioco non ci sono ancora razze.'));
        }
        links.push({ kind: 'section', label: say(lang, 'Полка карт', 'Полиця карт', 'Card shelf', 'Estante de cartas', 'Kartenregal', 'Scaffale delle carte'), href: '/battles' });
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
              `La obra más antigua es ${quote(workName(first))}, de ${first.year}.`,
              `Das früheste Werk ist ${quote(workName(first))}, von ${first.year}.`,
              `L’opera più antica è ${quote(workName(first))}, del ${first.year}.`,
            ),
          );
          links.push(workLink(first, lang));
        } else {
          paragraphs.push(say(lang, 'У работ не указан год.', 'У робіт не вказано рік.', 'The works have no year set.', 'Las obras no tienen año indicado.', 'Bei den Werken ist kein Jahr angegeben.', 'Le opere non hanno l’anno indicato.'));
        }
        break;
      }
    }
  }

  if (parts.length) {
    paragraphs.unshift(
      say(lang, 'Сейчас на сайте: ', 'Зараз на сайті: ', 'On the site now: ', 'Ahora en el sitio: ', 'Jetzt auf der Seite: ', 'Ora sul sito: ') + joinList(lang, parts) + '.',
    );
  }
  return { intent: 'count', paragraphs, links: links.slice(0, LINKS_MAX) };
}

// ── Каталог ответов на сервере ──────────────────────────────────────────────
//
// Порядок такой: правила, которым нельзя ошибиться (страница, команда, подбор,
// приветствие), идут первыми; затем вопрос сравнивается по смыслу с каталогом
// (`POST /ask/route`); и только если каталог не уверен, отвечают прежние правила
// по словам. Когда у каталога нет уверенного ответа, он предлагает три
// ближайших вопроса кнопками.

/** Вопросы, которые разбирают правила и каталога не спрашивают: у них точные
 *  приметы, а смысловое сходство для них не нужно. */
export function ownedByRules(intent: AskIntent): boolean {
  return intent === 'page' || intent === 'go' || intent === 'match' || intent === 'chat';
}

/** Виды ответа по словам, которые срабатывают на одно слово и поэтому ошибаются
 *  чаще прочих («сколько делается работа» — это не «сколько работ»). Под таким
 *  ответом каталог дописывает свои предложения. */
const LOOSE: AskIntent[] = ['summary', 'count', 'new', 'tale', 'play'];

export function isLoose(intent: AskIntent): boolean {
  return LOOSE.includes(intent);
}

/** Ответ из данных сайта по записи каталога без текста. */
export function dataReply(index: AskIndex, lang: Lang, handler: AskHandler, question: string, gazetteTotal: number | null): AskReply {
  const parsed = parseAsk(question);
  switch (handler) {
    case 'count':
      return countReply(index, lang, parsed.intent === 'count' ? parsed.facts ?? [] : [], gazetteTotal);
    case 'new':
      return newReply(index, lang);
    case 'read':
      return taleReply(index, lang, parsed.minutes);
    case 'start':
      return startReply(index, lang);
  }
}

/** Ответ из готового текста записи каталога. */
export function textReply(route: NonNullable<AskRouteReply['answer']>): AskReply | null {
  const paragraphs = route.text.split(/\n{2,}/).map((p) => p.trim()).filter(Boolean);
  if (!paragraphs.length) return null;
  const links: AskLink[] = route.links.map((l) => ({ kind: 'section', label: l.label, href: l.href }));
  return { intent: 'catalog', paragraphs, links: links.slice(0, LINKS_MAX) };
}

/** Предложенные каталогом вопросы кнопками. */
export function suggestionLinks(route: AskRouteReply | null, asked = ''): AskLink[] {
  const same = sameQuestion(asked);
  return (route?.candidates ?? [])
    .filter((c) => !same(c.question))
    .map((c) => ({ kind: 'question' as const, label: c.question, href: '' }));
}

/** Вопрос узнан, но записи каталога ещё нечего сказать. Честнее сказать об этом,
 *  чем отдать вопрос соседней записи. Помечен `missed`: такие вопросы и показывают,
 *  какие ответы пора написать. */
export function unansweredReply(lang: Lang): AskReply {
  return {
    intent: 'catalog',
    missed: true,
    paragraphs: [
      say(
        lang,
        'Об этом на сайте пока не написано. Лучше спросить мастера напрямую: кнопка «Написать» в шапке.',
        'Про це на сайті поки не написано. Краще запитати майстра напряму: кнопка «Написати» в шапці.',
        'This is not written on the site yet. It is better to ask the master directly: the “Write” button in the header.',
        'Esto aún no está escrito en el sitio. Mejor preguntar directamente a la autora: el botón «Escribir» de la cabecera.',
        'Das steht noch nicht auf der Seite. Am besten fragst du die Autorin direkt: Schaltfläche „Schreiben“ in der Kopfzeile.',
        'Questo non è ancora scritto sul sito. Meglio chiedere direttamente all’autrice: il pulsante «Scrivi» nell’intestazione.',
      ),
    ],
    links: [{ kind: 'section', label: say(lang, 'Об авторе', 'Про автора', 'About the author', 'Sobre la autora', 'Über die Autorin', 'L’autrice'), href: '/author' }],
  };
}

function chatReply(lang: Lang, kind: 'hello' | 'thanks'): AskReply {
  const hello = say(
    lang,
    'Здравствуйте! Спросите о работах, небылицах или игре — или выберите вопрос ниже.',
    'Вітаю! Запитайте про роботи, небилиці чи гру — або оберіть запитання нижче.',
    'Hello! Ask about the works, the tales or the game — or pick a question below.',
    '¡Hola! Pregunta por las obras, los cuentos o el juego, o elige una pregunta abajo.',
    'Hallo! Frag nach den Werken, den Geschichten oder dem Spiel – oder wähle unten eine Frage.',
    'Ciao! Chiedi delle opere, dei racconti o del gioco, oppure scegli una domanda qui sotto.',
  );
  const thanks = say(
    lang,
    'Пожалуйста. Если есть ещё вопрос — спрашивайте.',
    'Будь ласка. Якщо є ще запитання — питайте.',
    'You are welcome. If you have another question, ask.',
    'De nada. Si tienes otra pregunta, adelante.',
    'Gern geschehen. Wenn du noch eine Frage hast, frag einfach.',
    'Prego. Se hai un’altra domanda, chiedi pure.',
  );
  return { intent: 'chat', paragraphs: [kind === 'hello' ? hello : thanks], links: kind === 'hello' ? nearestQuestions(lang, '') : [] };
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
  /** Ответы подбора работы: пришли с кнопки выбора, а не с набранного вопроса. */
  quiz?: string[];
}

export function answer(index: AskIndex, query: string, lang: Lang, ctx: AskContext): AskReply {
  if (ctx.quiz) return matchReply(index, lang, ctx.quiz);
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
        ? asQuestion(searchReply(index, lang, parsed.topic, null, 'tales', 'tale'), lang, query)
        : taleReply(index, lang, parsed.minutes);
    case 'play':
      return parsed.topic
        ? asQuestion(searchReply(index, lang, parsed.topic, null, 'cards', 'play'), lang, query)
        : playReply(index, lang);
    case 'match':
      return matchStart(lang);
    case 'go':
      return goReply(index, lang, parsed.go as GoSpec);
    case 'chat':
      return chatReply(lang, parsed.chat ?? 'hello');
    case 'page':
      return pageReply(index, lang, ctx.page, ctx.text);
    case 'about':
      return aboutReply(index, lang, parsed.about as AboutKey, !!parsed.price, ctx.text);
    case 'count':
      return countReply(index, lang, parsed.facts ?? [], ctx.gazetteTotal);
    case 'search':
      return asQuestion(searchReply(index, lang, parsed.topic, ctx.meaning, 'all', 'search'), lang, query);
    default:
      return noneReply(lang, '', query);
  }
}
