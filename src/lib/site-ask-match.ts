// «Подбери мне работу»: три вопроса и три работы.
//
// Настроение работы узнаётся по словам в её названии и описании и по расе её
// карты, а не по «чутью»: колдовское — ведьма, маг, шаман, метла; тёмное —
// вампир, призрак, Франкенштейн, нежить; сказочное — бабушка, гном, лесник,
// портной; литературное — Гамлет, Квазимодо, Эсмеральда, мушкетёр. Работа, о
// которой слова ничего не говорят, ни в одно настроение не попадает, и это
// честно: лучше три подходящих, чем три случайных.
//
// Только разбор и сборка ответа; шаги и кнопки рисует окно.
import { workBase } from '$lib/workText';
import { normalizeQuery } from '$lib/keeper-search';
import { cardOfWork } from '$lib/world';
import type { AskIndex } from '$lib/site-index';
import type { BattleCard, FigurineListItem } from '$lib/types/api';
import { cardRace } from '$lib/site-search';
import { say, quote, workLink, workName, newest, LINKS_MAX, type AskLink, type AskReply, type Lang, type Said } from '$lib/site-ask-kit';

const MATCH_PHRASES = [
  'подбери', 'подобрать', 'подбор', 'посоветуй работу', 'посоветуй мне работу', 'выбери мне', 'что мне выбрать', 'помоги выбрать',
  'pick a work', 'pick me', 'recommend a work', 'choose a work', 'find me a work', 'help me choose',
  'підбери', 'підібрати', 'порадь роботу', 'допоможи вибрати',
  'elige una obra', 'elígeme', 'eligeme', 'recomiéndame una obra', 'recomiendame una obra', 'ayúdame a elegir', 'ayudame a elegir',
  'such mir ein werk', 'such mir eine', 'empfiehl mir', 'hilf mir', 'wähle ein werk',
  'scegli un’opera', "scegli un'opera", 'scegli per me', 'consigliami', 'aiutami a scegliere',
];

export const wantsMatch = (query: string) => {
  const q = normalizeQuery(query);
  return MATCH_PHRASES.some((p) => q.includes(p));
};

type Mood = 'witch' | 'dark' | 'tale' | 'book';

const MOOD_WORDS: Record<Mood, string[]> = {
  witch: ['witch', 'wizard', 'mage', 'shaman', 'yaga', 'spell', 'broom', 'ведьм', 'відьм', 'маг', 'шаман', 'яга', 'метл', 'мітл', 'чар', 'bruj', 'hexe', 'streg', 'escob', 'besen', 'scopa'],
  dark: ['vampire', 'ghost', 'frankenstein', 'creature', 'zombie', 'skeleton', 'dead', 'nosferatu', 'plague', 'вампир', 'вампір', 'призрак', 'привид', 'франкенш', 'нежит', 'чудовищ', 'скелет', 'vampir', 'fantasm', 'gespenst', 'esquelet', 'skelett', 'schelet'],
  tale: ['granny', 'grandma', 'gnome', 'forester', 'tailor', 'cooking', 'kitchen', 'riding hood', 'piper', 'book', 'fairy', 'mermaid', 'baker', 'бабуш', 'гном', 'лесник', 'лісник', 'портн', 'кравц', 'повар', 'кухн', 'шапочк', 'русалк', 'книг', 'сказ', 'казк', 'abuel', 'nonn', 'oma ', 'gnom', 'cuento', 'märchen', 'fiab', 'siren'],
  book: ['hamlet', 'quazimodo', 'esmeralda', 'notre', 'musketeer', 'porthos', 'poe', 'edgar', 'potter', 'agatha', 'lady', 'dracula', 'гамлет', 'квазимод', 'эсмеральд', 'есмеральд', 'мушкет', 'портос', 'эдгар', 'едгар', 'поттер', 'агат', 'mosquet', 'musketier', 'moschett', 'jorob', 'glöckner', 'gobbo'],
};

/** Что за раса у карты → в какое настроение она тянет. */
const RACE_MOOD: [string[], Mood][] = [
  [['ведьм', 'відьм', 'witch', 'маг', 'mage', 'wizard', 'bruj', 'hexe', 'streg', 'mago', 'magi', 'zauber'], 'witch'],
  [['нежит', 'undead', 'no muert', 'untot', 'non mort'], 'dark'],
  [['гном', 'gnome', 'люди', 'human', 'водян', 'water', 'merfolk', 'gnom', 'zwerg', 'human', 'mensch', 'uman', 'agua', 'wasser', 'acqua'], 'tale'],
];

const ANIMAL_WORDS = ['gato', 'gatt', 'katze', 'pájar', 'pajar', 'vogel', 'uccell', 'perro', 'hund', 'cane', 'gallin', 'huhn', 'cuervo', 'rabe', 'corvo', 'búho', 'eule', 'gufo',
  'cat', 'bird', 'sparrow', 'hen', 'chicken', 'dog', 'snake', 'mice', 'mouse', 'squirrel', 'monkey', 'piglet', 'raven', 'owl', 'falcon', 'hawk', 'snail',
  'кот', 'кош', 'птиц', 'птах', 'воробь', 'горобц', 'кур', 'цыпл', 'собак', 'пес', 'пёс', 'змей', 'змі', 'мыш', 'миш', 'белк', 'обезьян', 'мавп', 'поросен', 'ворон', 'сов', 'сокол', 'сокіл', 'улитк', 'равлик'];

const has = (text: string, words: string[]) => words.some((w) => text.includes(w));

function textOf(w: FigurineListItem, card: BattleCard | null): string {
  return normalizeQuery([w.name, w.shortText, workBase(w, 'name'), workBase(w, 'shortText'), card?.titleRu, card?.titleEn, card?.titleUk, card?.titleEs, card?.titleDe, card?.titleIt, card?.figurineName].filter(Boolean).join(' '));
}

function raceText(card: BattleCard | null): string {
  return normalizeQuery([card?.raceNameRu, card?.raceNameEn, card?.raceNameUk, card?.raceNameEs, card?.raceNameDe, card?.raceNameIt].filter(Boolean).join(' '));
}

function moodScore(w: FigurineListItem, card: BattleCard | null, mood: Mood): number {
  const name = normalizeQuery(w.name);
  const text = textOf(w, card);
  let score = 0;
  if (has(name, MOOD_WORDS[mood])) score += 3;
  else if (has(text, MOOD_WORDS[mood])) score += 2;
  const race = raceText(card);
  for (const [stems, m] of RACE_MOOD) if (m === mood && has(race, stems)) score += 3;
  return score;
}

export interface Pick {
  work: FigurineListItem;
  score: number;
  reasons: string[];
}

const MOODS: Mood[] = ['witch', 'dark', 'tale', 'book'];

function moodLabel(lang: Lang, m: Mood): string {
  switch (m) {
    case 'witch': return say(lang, 'колдовское', 'чаклунське', 'witchy', 'brujeril', 'hexenhaft', 'stregonesco');
    case 'dark': return say(lang, 'тёмное', 'темне', 'dark', 'oscuro', 'düster', 'cupo');
    case 'tale': return say(lang, 'сказочное', 'казкове', 'fairy-tale', 'de cuento de hadas', 'märchenhaft', 'fiabesco');
    case 'book': return say(lang, 'литературное', 'літературне', 'literary', 'literario', 'literarisch', 'letterario');
  }
}

export function pickWorks(index: AskIndex, lang: Lang, answers: string[]): Pick[] {
  const mood = (answers.find((a) => a.startsWith('mood:'))?.slice(5) ?? '') as Mood | '';
  const animals = answers.includes('animals:yes');
  const inHouse = answers.includes('house:yes');
  const world = { cards: index.cards, frames: [], etudes: index.etudes, tales: index.tales };

  const picks: Pick[] = [];
  for (const w of index.works) {
    if (inHouse && w.status !== 'available') continue;
    const card = cardOfWork(world, w.id);
    const reasons: string[] = [];
    let score = 0;
    if (mood) {
      const s = moodScore(w, card, mood);
      if (s > 0) {
        score += s;
        reasons.push(moodLabel(lang, mood));
      }
    }
    if (animals && has(textOf(w, card), ANIMAL_WORDS)) {
      score += 2;
      reasons.push(say(lang, 'со зверьком или птицей', 'зі звірятком або птахом', 'with a pet or a bird', 'con un animalito o un pájaro', 'mit einem Tier oder Vogel', 'con un animaletto o un uccello'));
    }
    // Работа с картой в игре чуть выше: о ней можно прочесть и сыграть.
    if (score > 0 && card) score += 0.5;
    picks.push({ work: w, score, reasons });
  }
  const ranked = picks.filter((p) => p.score > 0).sort((a, b) => b.score - a.score);
  if (ranked.length >= 3) return ranked.slice(0, 3);
  // Слова мало что нашли: добавляем самые свежие, и ответ скажет об этом.
  const fresh = newest(picks.filter((p) => p.score === 0), (p) => p.work.createdAt).slice(0, 3 - ranked.length);
  return [...ranked, ...fresh];
}

// ── Шаги ──────────────────────────────────────────────────────────────────

interface Choice {
  value: string;
  label: Said;
}

const STEPS: { key: string; ask: Said; choices: Choice[] }[] = [
  {
    key: 'mood',
    ask: ['Подберу по трём вопросам. Какое настроение вам ближе?', 'Підберу за трьома питаннями. Який настрій вам ближчий?', 'Three questions, then three works. Which mood is closer to you?',
      'Tres preguntas y tres obras. ¿Qué ambiente te atrae más?', 'Drei Fragen, dann drei Werke. Welche Stimmung liegt dir näher?', 'Tre domande, poi tre opere. Quale atmosfera senti più vicina?'],
    choices: [
      { value: 'witch', label: ['Колдовское', 'Чаклунське', 'Witchy', 'Brujeril', 'Hexenhaft', 'Stregonesca'] },
      { value: 'dark', label: ['Тёмное и жуткое', 'Темне й моторошне', 'Dark and eerie', 'Oscuro e inquietante', 'Düster und unheimlich', 'Cupa e inquietante'] },
      { value: 'tale', label: ['Сказочное и тёплое', 'Казкове й тепле', 'Fairy-tale and warm', 'De cuento y cálido', 'Märchenhaft und warm', 'Fiabesca e calda'] },
      { value: 'book', label: ['Литературное', 'Літературне', 'Literary', 'Literario', 'Literarisch', 'Letteraria'] },
    ],
  },
  {
    key: 'animals',
    ask: ['Второй вопрос: нужен ли рядом с героиней зверёк или птица?', 'Друге питання: чи потрібен поруч із героїнею звірятко або птах?', 'Second: should the figure come with a pet or a bird?',
      'Segunda: ¿quieres que la figura vaya con un animalito o un pájaro?', 'Zweitens: Soll die Figur ein Tier oder einen Vogel dabeihaben?', 'Seconda: vuoi che la statuina abbia con sé un animaletto o un uccello?'],
    choices: [
      { value: 'yes', label: ['Да, со зверьком или птицей', 'Так, зі звірятком або птахом', 'Yes, with a pet or a bird', 'Sí, con un animalito o un pájaro', 'Ja, mit Tier oder Vogel', 'Sì, con un animaletto o un uccello'] },
      { value: 'any', label: ['Всё равно', 'Байдуже', 'No preference', 'Me da igual', 'Egal', 'Indifferente'] },
    ],
  },
  {
    key: 'house',
    ask: ['Последний вопрос: показывать только те работы, что ещё в доме?', 'Останнє питання: показувати лише ті роботи, що ще в домі?', 'Last: show only the works that are still in the house?',
      'Última: ¿mostrar solo las obras que siguen disponibles?', 'Zuletzt: Nur die Werke zeigen, die noch verfügbar sind?', 'Ultima: mostrare solo le opere ancora disponibili?'],
    choices: [
      { value: 'yes', label: ['Только те, что ещё в доме', 'Лише ті, що ще в домі', 'Only those still in the house', 'Solo las disponibles', 'Nur verfügbare', 'Solo quelle disponibili'] },
      { value: 'any', label: ['Любые', 'Будь-які', 'Any', 'Cualquiera', 'Alle', 'Tutte'] },
    ],
  },
];

function choiceLinks(lang: Lang, step: number, answers: string[]): AskLink[] {
  const s = STEPS[step];
  return s.choices.map((c) => ({
    kind: 'choice' as const,
    label: say(lang, ...c.label),
    href: '',
    quiz: [...answers, `${s.key}:${c.value}`],
  }));
}

/** Первый вопрос — когда подбор только что попросили. */
export function matchStart(lang: Lang): AskReply {
  return { intent: 'match', paragraphs: [say(lang, ...STEPS[0].ask)], links: choiceLinks(lang, 0, []) };
}

/** Ответ на очередной выбор: следующий вопрос или три работы. */
export function matchReply(index: AskIndex, lang: Lang, answers: string[]): AskReply {
  if (answers.length < STEPS.length) {
    return { intent: 'match', paragraphs: [say(lang, ...STEPS[answers.length].ask)], links: choiceLinks(lang, answers.length, answers) };
  }
  const picks = pickWorks(index, lang, answers);
  if (!picks.length) {
    return {
      intent: 'match',
      paragraphs: [say(lang, 'Под такие условия работ сейчас нет. Попробуйте ответить мягче.', 'Під такі умови робіт зараз немає. Спробуйте відповісти м’якше.', 'No works match those answers right now. Try softer answers.', 'Ahora mismo ninguna obra encaja con esas respuestas. Prueba respuestas menos estrictas.', 'Zu diesen Antworten passt gerade kein Werk. Versuch es mit weniger strengen Antworten.', 'Al momento nessuna opera corrisponde a queste risposte. Prova risposte meno rigide.')],
      links: choiceLinks(lang, 0, []),
    };
  }
  const matched = picks.filter((p) => p.score > 0);
  const names = matched.map((p) => quote(workName(p.work)));
  const lead = matched.length
    ? say(lang, `Вот что подходит: ${names.join(', ')}.`, `Ось що підходить: ${names.join(', ')}.`, `These fit: ${names.join(', ')}.`, `Encajan: ${names.join(', ')}.`, `Es passen: ${names.join(', ')}.`, `Corrispondono: ${names.join(', ')}.`)
    : say(lang, 'Точно подходящих нет — вот самые свежие работы.', 'Точно підхожих немає — ось найсвіжіші роботи.', 'Nothing fits exactly — here are the newest works.', 'Nada encaja del todo; estas son las obras más recientes.', 'Nichts passt genau – hier sind die neuesten Werke.', 'Niente corrisponde del tutto: ecco le opere più recenti.');
  const note = matched.length && matched.length < picks.length
    ? say(lang, 'Остальное — самые свежие работы: слова о них мало что говорят.', 'Решта — найсвіжіші роботи: слова про них мало що кажуть.', 'The rest are the newest works: the words say little about them.', 'Las demás son las obras más recientes: las palabras dicen poco de ellas.', 'Die übrigen sind die neuesten Werke: Die Wörter sagen wenig über sie.', 'Le altre sono le opere più recenti: le parole dicono poco su di loro.')
    : '';
  const links: AskLink[] = picks.map((p) =>
    workLink(p.work, lang, p.reasons.length ? p.reasons.join(' · ') : say(lang, 'новая работа', 'нова робота', 'newest work', 'obra más reciente', 'neuestes Werk', 'opera più recente')),
  );
  links.push({
    kind: 'question',
    label: say(lang, 'Подбери мне работу', 'Підбери мені роботу', 'Pick a work for me', 'Elige una obra para mí', 'Such mir ein Werk aus', 'Scegli un’opera per me'),
    href: '',
  });
  return { intent: 'match', paragraphs: [lead, note].filter(Boolean), links: links.slice(0, LINKS_MAX) };
}
