// «О чём эта страница»: ответ про то, что сейчас открыто у человека.
//
// Страницу называет адрес: небылица, лист вестника, работа или карта на полке.
// Про небылицу и лист — краткое содержание из самого текста (`taleText.ts`), про
// работу — материал, размер и всё, что с ней связано: небылица, карта, этюды.
// Ничего не придумано: каждая строка ответа есть на сайте.
import { workBase } from '$lib/workText';
import { localized } from '$lib/i18n';
import { normalizeQuery } from '$lib/keeper-search';
import { leafCopy } from '$lib/gazette';
import { readingMinutes } from '$lib/tales';
import { cardOfWork, etudesWithCard, talesOfWork, type World } from '$lib/world';
import { cardRace, cardTitle } from '$lib/site-search';
import { summarizeTale, talePlain } from '$lib/taleText';
import type { AskIndex } from '$lib/site-index';
import type { BattleCard, FigurineListItem, GazetteLeaf } from '$lib/types/api';
import {
  say, joinList, quote, short_min, n_minutes, workName, workLink, taleLink, leafLink, cardLink, etudeLink,
  LINKS_MAX, MATERIALS, type AskLink, type AskReply, type Lang,
} from '$lib/site-ask-kit';

/** Где человек сейчас: путь и, на полке карт, выбранная карта. */
export interface AskPage {
  path: string;
  card: string | null;
}

export type PageTarget =
  | { kind: 'tale' | 'leaf'; slug: string }
  | { kind: 'work'; slug: string }
  | { kind: 'card'; id: string };

export function pageTarget(page: AskPage | undefined): PageTarget | null {
  if (!page) return null;
  const seg = page.path.split('/').filter(Boolean);
  if (seg[0] === 'tales' && seg[1]) return { kind: 'tale', slug: seg[1] };
  if (seg[0] === 'gazette' && seg[1]) return { kind: 'leaf', slug: seg[1] };
  if (seg[0] === 'figurines' && seg[1]) return { kind: 'work', slug: seg[1] };
  if (seg[0] === 'battles' && !seg[1] && page.card) return { kind: 'card', id: page.card };
  return null;
}

export const hasPageContext = (page: AskPage | undefined) => pageTarget(page) !== null;

const PAGE_PHRASES = [
  'эта страница', 'эту страницу', 'этой странице', 'о чем это', 'о чем эта', 'о чем здесь', 'про что эта', 'про что это', 'перескажи',
  'расскажи про эту', 'расскажи об этой', 'расскажи про нее', 'что за работа', 'об этой работе', 'об этой небылице', 'об этой карте',
  'this page', 'what is this page', 'this tale', 'this work', 'this card', 'summarize this', 'summarise this', 'sum up', "what's this about", 'what is this about',
  'ця сторінка', 'цю сторінку', 'цій сторінці', 'про що ця', 'про що це', 'розкажи про цю', 'розкажи про неї', 'переказ',
  'esta página', 'esta pagina', 'este cuento', 'esta obra', 'esta carta', 'de qué trata esto', 'de que trata esto', 'resume esto',
  'diese seite', 'dieser seite', 'diese geschichte', 'dieses werk', 'diese karte', 'worum geht es hier', 'fasse zusammen',
  'questa pagina', 'questo racconto', 'quest’opera', "quest'opera", 'questa carta', 'di cosa parla questo', 'riassumi questo',
];
const PAGE_WORDS = ['страниц', 'сторінк', 'page', 'página', 'pagina', 'seite'];
const PAGE_ASKS = ['суммар', 'кратк', 'коротк', 'перескаж', 'о чем', 'про что', 'summar', 'what is', 'about', 'стисл', 'підсум', 'про що', 'расскажи', 'розкажи', 'tell',
  'resum', 'de qué', 'de que', 'cuéntame', 'cuentame', 'zusammenfass', 'worum', 'erzähl', 'riassum', 'di cosa', 'raccontami'];

export function wantsPage(query: string): boolean {
  const q = normalizeQuery(query);
  if (PAGE_PHRASES.some((p) => q.includes(p))) return true;
  return PAGE_WORDS.some((w) => q.includes(w)) && PAGE_ASKS.some((a) => q.includes(a));
}

function workMaterials(w: FigurineListItem, lang: Lang): string[] {
  return workBase(w, 'material')
    .split(',')
    .map((m) => m.trim().toLowerCase())
    .filter((m) => MATERIALS[m])
    .map((m) => say(lang, MATERIALS[m][0], MATERIALS[m][1], MATERIALS[m][2], MATERIALS[m][3], MATERIALS[m][4], MATERIALS[m][5]));
}

function workHeight(w: FigurineListItem): string | null {
  const m = workBase(w, 'dimensions').match(/(\d+(?:[.,]\d+)?)(?:\s*[-–]\s*(\d+(?:[.,]\d+)?))?\s*(?:cm|см)/i);
  if (!m) return null;
  return m[2] ? `${m[1]}–${m[2]}` : m[1];
}

function statusKey(w: FigurineListItem): string {
  switch (w.status) {
    case 'available': return 'figurineStatusAvailable';
    case 'reserved': return 'figurineStatusReserved';
    case 'in_progress': return 'figurineStatusInProgress';
    default: return 'figurineStatusSold';
  }
}

function worldOf(index: AskIndex): World {
  return { cards: index.cards, frames: [], etudes: index.etudes, tales: index.tales };
}

/** Что связано с работой: небылицы, карта, этюды, где она стоит. */
function related(index: AskIndex, lang: Lang, workId: string | null | undefined): { text: string; links: AskLink[] } {
  const world = worldOf(index);
  const tales = talesOfWork(world, workId);
  const card = cardOfWork(world, workId);
  const etudes = card ? etudesWithCard(world, card) : [];
  const bits: string[] = [];
  const links: AskLink[] = [];
  if (tales[0]) {
    const own = leafCopy(tales[0], lang);
    const min = readingMinutes(own.body);
    bits.push(
      say(lang, `небылица ${quote(own.title)}${min ? ` (${short_min(lang, min)})` : ''}`, `небилиця ${quote(own.title)}${min ? ` (${short_min(lang, min)})` : ''}`, `the tall tale ${quote(own.title)}${min ? ` (${short_min(lang, min)})` : ''}`, `el cuento ${quote(own.title)}${min ? ` (${short_min(lang, min)})` : ''}`, `die Geschichte ${quote(own.title)}${min ? ` (${short_min(lang, min)})` : ''}`, `il racconto ${quote(own.title)}${min ? ` (${short_min(lang, min)})` : ''}`),
    );
    links.push(taleLink(tales[0], lang));
  }
  if (card) {
    bits.push(say(lang, `карта ${quote(cardTitle(card, lang))} в игре`, `карта ${quote(cardTitle(card, lang))} у грі`, `the card ${quote(cardTitle(card, lang))} in the game`, `la carta ${quote(cardTitle(card, lang))} en el juego`, `die Karte ${quote(cardTitle(card, lang))} im Spiel`, `la carta ${quote(cardTitle(card, lang))} nel gioco`));
    links.push(cardLink(card, lang));
  }
  for (const e of etudes.slice(0, 2)) links.push(etudeLink(e, lang));
  return { text: bits.length ? joinList(lang, bits) : '', links };
}

function workPage(index: AskIndex, lang: Lang, w: FigurineListItem, text: (k: string) => string): AskReply {
  const mats = workMaterials(w, lang);
  const height = workHeight(w);
  const first = say(
    lang,
    `${quote(workName(w))} — работа${w.year ? ` ${w.year} года` : ''}. Сейчас: ${text(statusKey(w)).toLocaleLowerCase('ru')}.`,
    `${quote(workName(w))} — робота${w.year ? ` ${w.year} року` : ''}. Зараз: ${text(statusKey(w)).toLocaleLowerCase('uk')}.`,
    `${quote(workName(w))} is a work${w.year ? ` from ${w.year}` : ''}. Status: ${text(statusKey(w)).toLocaleLowerCase('en')}.`,
    `${quote(workName(w))} es una obra${w.year ? ` de ${w.year}` : ''}. Estado: ${text(statusKey(w)).toLocaleLowerCase('es')}.`,
    `${quote(workName(w))} ist ein Werk${w.year ? ` von ${w.year}` : ''}. Stand: ${text(statusKey(w))}.`,
    `${quote(workName(w))} è un’opera${w.year ? ` del ${w.year}` : ''}. Stato: ${text(statusKey(w)).toLocaleLowerCase('it')}.`,
  );
  const made: string[] = [];
  if (mats.length) made.push(say(lang, `Материалы: ${joinList(lang, mats)}.`, `Матеріали: ${joinList(lang, mats)}.`, `Materials: ${joinList(lang, mats)}.`, `Materiales: ${joinList(lang, mats)}.`, `Materialien: ${joinList(lang, mats)}.`, `Materiali: ${joinList(lang, mats)}.`));
  if (height) made.push(say(lang, `Высота около ${height} см.`, `Висота близько ${height} см.`, `About ${height} cm tall.`, `Mide unos ${height} cm.`, `Etwa ${height} cm hoch.`, `Alta circa ${height} cm.`));
  const rel = related(index, lang, w.id);
  const tail = rel.text
    ? say(lang, `О ней есть ${rel.text}.`, `Про неї є ${rel.text}.`, `It comes with ${rel.text}.`, `La acompaña ${rel.text}.`, `Dazu gehört ${rel.text}.`, `La accompagna ${rel.text}.`)
    : say(lang, 'Небылиц и карты у этой работы пока нет.', 'Небилиць і карти в цієї роботи поки немає.', 'This work has no tall tale or card yet.', 'Esta obra aún no tiene cuento ni carta.', 'Zu diesem Werk gibt es noch keine Geschichte und keine Karte.', 'Quest’opera non ha ancora né racconto né carta.');
  return { intent: 'page', paragraphs: [first, made.join(' '), tail].filter(Boolean), links: rel.links.slice(0, LINKS_MAX) };
}

function leafPage(index: AskIndex, lang: Lang, leaf: GazetteLeaf, isTale: boolean): AskReply {
  const own = leafCopy(leaf, lang);
  const plain = talePlain(own.body);
  const min = readingMinutes(own.body);
  const summary = summarizeTale(plain.paragraphs, plain.quotes);

  const kind = isTale
    ? say(lang, 'Небылица', 'Небилиця', 'The tall tale', 'El cuento', 'Die Geschichte', 'Il racconto')
    : say(lang, 'Лист вестника', 'Аркуш вісника', 'The gazette leaf', 'La hoja de la Gaceta', 'Das Blatt der Zeitung', 'Il foglio della Gazzetta');
  const head =
    `${kind} ${quote(own.title)}` +
    (min
      ? say(lang, `, ${n_minutes(lang, min)} чтения.`, `, ${n_minutes(lang, min)} читання.`, `, ${n_minutes(lang, min)} to read.`, `, ${n_minutes(lang, min)} de lectura.`, `, ${n_minutes(lang, min)} Lesezeit.`, `, ${n_minutes(lang, min)} di lettura.`)
      : '.');

  const paragraphs = [head];
  if (summary.sentences.length) {
    paragraphs.push(say(lang, 'Коротко, словами самого текста: ', 'Коротко, словами самого тексту: ', 'In brief, in the text’s own words: ', 'En pocas palabras, con las del propio texto: ', 'Kurz, mit den Worten des Textes selbst: ', 'In breve, con le parole del testo stesso: ') + summary.sentences.join(' '));
  } else if (own.dek) {
    paragraphs.push(own.dek);
  }
  if (summary.quote) {
    paragraphs.push(say(lang, `Реплика, которую автор выделил: ${quote(summary.quote)}`, `Репліка, яку автор виділив: ${quote(summary.quote)}`, `A line the author set apart: ${quote(summary.quote)}`, `Una frase que la autora destacó aparte: ${quote(summary.quote)}`, `Ein Satz, den die Autorin eigens abgesetzt hat: ${quote(summary.quote)}`, `Una frase che l’autrice ha messo a parte: ${quote(summary.quote)}`));
  }

  const links: AskLink[] = [];
  const work = leaf.figurineId ? index.works.find((w) => w.id === leaf.figurineId) : null;
  if (work) links.push(workLink(work, lang, say(lang, 'о ней эта история', 'про неї ця історія', 'this story is about her', 'esta historia trata de ella', 'diese Geschichte handelt von ihr', 'questa storia parla di lei')));
  const rel = related(index, lang, leaf.figurineId);
  for (const l of rel.links) if (l.kind === 'card' || l.kind === 'etude') links.push(l);
  return { intent: 'page', paragraphs, links: links.slice(0, LINKS_MAX) };
}

function cardPage(index: AskIndex, lang: Lang, card: BattleCard): AskReply {
  const race = cardRace(card, lang);
  const effect = (lang === 'ru' ? card.effectRu : lang === 'uk' ? card.effectUk : lang === 'en' ? card.effectEn : localized(card, 'effect', lang))?.trim() ?? '';
  const first = say(
    lang,
    `Карта ${quote(cardTitle(card, lang))}${race ? `, раса — ${race}` : ''}, ранг ${card.tier}.`,
    `Карта ${quote(cardTitle(card, lang))}${race ? `, раса — ${race}` : ''}, ранг ${card.tier}.`,
    `The card ${quote(cardTitle(card, lang))}${race ? `, race: ${race}` : ''}, rank ${card.tier}.`,
    `La carta ${quote(cardTitle(card, lang))}${race ? `, raza: ${race}` : ''}, rango ${card.tier}.`,
    `Die Karte ${quote(cardTitle(card, lang))}${race ? `, Volk: ${race}` : ''}, Rang ${card.tier}.`,
    `La carta ${quote(cardTitle(card, lang))}${race ? `, razza: ${race}` : ''}, rango ${card.tier}.`,
  );
  const stats = say(
    lang,
    `Здоровье ${card.health}, мана ${card.mana}, броня ${card.armor}.`,
    `Здоров’я ${card.health}, мана ${card.mana}, броня ${card.armor}.`,
    `Health ${card.health}, mana ${card.mana}, armor ${card.armor}.`,
    `Salud ${card.health}, maná ${card.mana}, armadura ${card.armor}.`,
    `Leben ${card.health}, Mana ${card.mana}, Rüstung ${card.armor}.`,
    `Salute ${card.health}, mana ${card.mana}, armatura ${card.armor}.`,
  );
  const paragraphs = [first, stats];
  if (effect) paragraphs.push(say(lang, `Эффект: ${effect}`, `Ефект: ${effect}`, `Effect: ${effect}`, `Efecto: ${effect}`, `Wirkung: ${effect}`, `Effetto: ${effect}`));

  const links: AskLink[] = [];
  const work = card.figurineId ? index.works.find((w) => w.id === card.figurineId) : null;
  if (work) links.push(workLink(work, lang));
  const world = worldOf(index);
  for (const t of talesOfWork(world, card.figurineId).slice(0, 2)) links.push(taleLink(t, lang));
  for (const e of etudesWithCard(world, card).slice(0, 2)) links.push(etudeLink(e, lang));
  return { intent: 'page', paragraphs, links: links.slice(0, LINKS_MAX) };
}

function noPage(lang: Lang): AskReply {
  return {
    intent: 'page',
    paragraphs: [
      say(
        lang,
        'Откройте страницу работы, небылицы или карты — тогда я расскажу о ней. Пока могу коротко рассказать обо всём сайте.',
        'Відкрийте сторінку роботи, небилиці або карти — тоді я розповім про неї. Поки можу коротко розповісти про весь сайт.',
        'Open the page of a work, a tall tale or a card and I will tell you about it. Meanwhile I can describe the whole site.',
        'Abre la página de una obra, un cuento o una carta y te la describo. Mientras tanto puedo resumir todo el sitio.',
        'Öffne die Seite eines Werks, einer Geschichte oder einer Karte, dann erzähle ich davon. Bis dahin kann ich die ganze Seite kurz beschreiben.',
        'Apri la pagina di un’opera, di un racconto o di una carta e te ne parlo. Intanto posso descrivere in breve tutto il sito.',
      ),
    ],
    links: [
      {
        kind: 'question',
        label: say(lang, 'Коротко: что это за сайт?', 'Коротко: що це за сайт?', 'Briefly, what is this site?', 'En pocas palabras, ¿qué es este sitio?', 'Kurz: Was ist das für eine Seite?', 'In breve, che sito è questo?'),
        href: '',
      },
    ],
  };
}

export function pageReply(index: AskIndex, lang: Lang, page: AskPage | undefined, text: (k: string) => string): AskReply {
  const target = pageTarget(page);
  if (!target) return noPage(lang);
  switch (target.kind) {
    case 'tale': {
      const leaf = index.tales.find((l) => l.slug === target.slug);
      return leaf ? leafPage(index, lang, leaf, true) : noPage(lang);
    }
    case 'leaf': {
      const leaf = index.gazette.find((l) => l.slug === target.slug) ?? index.tales.find((l) => l.slug === target.slug);
      return leaf ? leafPage(index, lang, leaf, leaf.kind === 'tale') : noPage(lang);
    }
    case 'work': {
      const work = index.works.find((w) => w.slug === target.slug || w.id === target.slug);
      return work ? workPage(index, lang, work, text) : noPage(lang);
    }
    case 'card': {
      const card = index.cards.find((c) => c.id === target.id);
      return card ? cardPage(index, lang, card) : noPage(lang);
    }
  }
}
