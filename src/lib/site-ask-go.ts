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
  type AskAction, type AskReply, type Lang, type Said,
} from '$lib/site-ask-kit';

export type GoSpec =
  | { kind: 'page'; href: string; label: Said }
  | { kind: 'archive'; topic: string }
  | { kind: 'latest'; what: 'work' | 'tale' }
  | { kind: 'random'; what: 'work' | 'tale' }
  | { kind: 'lang'; lang: Lang };

const VERBS = [
  'открой', 'открыть', 'перейди', 'перейти', 'отведи', 'веди ', 'зайди', 'покажи страницу', 'покажи раздел',
  'go to', 'open ', 'take me', 'show me the',
  'відкрий', 'відкрити', 'відведи', 'зайди',
  'abre ', 'abrir', 'ir a ', 'llévame', 'llevame', 've a ', 'muéstrame la página', 'muestrame la pagina',
  'öffne', 'oeffne', 'geh zu', 'gehe zu', 'bring mich', 'zeig mir die seite',
  'apri', 'aprire', 'vai a', 'vai al', 'vai alla', 'portami', 'mostrami la pagina',
];

const SWITCH_VERBS = [
  'переключи', 'смени', 'поменяй', 'переведи', 'включи', 'switch', 'change', 'перемкни', 'змін', 'увімкни',
  'cambia', 'pon ', 'pasa a', 'wechsle', 'stell', 'umstellen', 'passa a', 'metti',
];
const LANG_STEMS: [Lang, string[]][] = [
  ['es', ['испан', 'іспан', 'spanish', 'espa', 'spanisch', 'spagnol']],
  ['de', ['немецк', 'німецьк', 'german', 'deutsch', 'alemán', 'aleman', 'tedesc']],
  ['it', ['итальян', 'італійськ', 'italian', 'italiano', 'italienisch']],
  ['en', ['англ', 'english', 'inglés', 'ingles', 'englisch', 'inglese']],
  ['uk', ['украин', 'укр', 'ukrain', 'українськ', 'ucrani', 'ukrainisch', 'ucrain']],
  ['ru', ['русск', 'russian', 'російськ', 'рос', 'ruso', 'russisch', 'russo']],
];

const PAGES: { stems: string[]; href: string; label: Said }[] = [
  // Студия раньше игры: «studio» содержит «stud», и иначе «открой студию» вело бы в этюды.
  { stems: ['студи', 'studio', 'estudio'], href: '/studio',
    label: ['Студия', 'Студія', 'Studio', 'Estudio', 'Studio', 'Studio'] },
  { stems: ['полку карт', 'полка карт', 'полку', 'shelf', 'полиц', 'estante', 'regal', 'scaffale'], href: '/battles',
    label: ['Полка карт', 'Полиця карт', 'Card shelf', 'Estante de cartas', 'Kartenregal', 'Scaffale delle carte'] },
  { stems: ['игр', 'этюд', 'бой', 'битв', 'game', 'stud', 'battle', 'гру', 'гра', 'етюд', 'бій', 'juego', 'étude', 'etude', 'spiel', 'etüde', 'gioco', 'battagl', 'combate', 'kampf'], href: '/battles/etude',
    label: ['Игра', 'Гра', 'The game', 'El juego', 'Das Spiel', 'Il gioco'] },
  { stems: ['карт', 'card', 'carta', 'karte'], href: '/battles',
    label: ['Полка карт', 'Полиця карт', 'Card shelf', 'Estante de cartas', 'Kartenregal', 'Scaffale delle carte'] },
  { stems: ['небыл', 'байк', 'tales', 'небил', 'cuentos', 'geschichten', 'raccont'], href: '/tales',
    label: ['Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'] },
  { stems: ['вестник', 'газет', 'gazette', 'вісник', 'gaceta', 'zeitung', 'gazzetta'], href: '/gazette',
    label: ['Вестник', 'Вісник', 'Gazette', 'Gaceta', 'Zeitung', 'Gazzetta'] },
  { stems: ['мастерск', 'workshop', 'майстерн', 'taller', 'werkstatt', 'laboratorio'], href: '/workshop',
    label: ['Мастерская', 'Майстерня', 'The workshop', 'El taller', 'Die Werkstatt', 'Il laboratorio'] },
  { stems: ['об авторе', 'автор', 'author', 'про автора', 'autora', 'autorin', 'autrice'], href: '/author',
    label: ['Об авторе', 'Про автора', 'About the author', 'Sobre la autora', 'Über die Autorin', 'L’autrice'] },
  { stems: ['приобрест', 'acquire', 'придбат', 'adquirir', 'erwerben', 'acquist'], href: '/acquire',
    label: ['Как приобрести работу', 'Як придбати роботу', 'How to acquire a work', 'Cómo adquirir una obra', 'Wie man ein Werk erwirbt', 'Come acquistare un’opera'] },
  { stems: ['профил', 'profile', 'профіл', 'кабинет', 'perfil', 'profil', 'profilo'], href: '/profile',
    label: ['Профиль', 'Профіль', 'Profile', 'Perfil', 'Profil', 'Profilo'] },
  { stems: ['политик', 'privacy', 'конфиденц', 'політик', 'privacidad', 'datenschutz'], href: '/privacy',
    label: ['Политика конфиденциальности', 'Політика конфіденційності', 'Privacy policy', 'Política de privacidad', 'Datenschutzerklärung', 'Informativa sulla privacy'] },
  { stems: ['главн', 'домой', 'home', 'головн', 'inicio', 'portada', 'startseite', 'principale'], href: '/',
    label: ['Главная', 'Головна', 'Home', 'Inicio', 'Startseite', 'Pagina principale'] },
  { stems: ['архив', 'работы', 'все работы', 'каталог', 'works', 'archive', 'gallery', 'архів', 'роботи', 'archivo', 'obras', 'archiv', 'werke', 'archivio', 'opere'], href: '/figurines',
    label: ['Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'] },
];

const WORK_WORDS = ['работ', 'фигур', 'кукл', 'work', 'figurin', 'doll', 'робот', 'фігур', 'ляльк', 'obra', 'figura', 'muñec', 'werk', 'figur', 'puppe', 'opera', 'statuin', 'bambol'];
const TALE_WORDS = ['небыл', 'байк', 'tale', 'story', 'небил', 'истори', 'cuento', 'historia', 'geschicht', 'raccont', 'storia'];
const LATEST_WORDS = ['последн', 'новую', 'новый', 'новое', 'свежую', 'свежий', 'latest', 'newest', 'остан', 'нову', 'últim', 'ultim', 'nuev', 'neuest', 'letzt', 'nuov', 'recient', 'recent'];
const RANDOM_WORDS = ['случайн', 'наугад', 'random', 'навмання', 'любую', 'будь-як', 'azar', 'aleatori', 'zufällig', 'zufaellig', 'irgendein', 'caso', 'casuale'];
const ARCHIVE_PHRASES = ['в архиве', 'в архив', 'in the archive', 'in archive', 'в архіві', 'в архів', 'en el archivo', 'im archiv', 'nell’archivio', "nell'archivio", 'in archivio'];
const ARCHIVE_NOISE = new Set([
  'архив', 'архиве', 'архів', 'архіві', 'archive', 'найди', 'найти', 'покажи', 'показать', 'открой', 'открыть', 'show', 'find', 'open', 'знайди', 'відкрий',
  'archivo', 'busca', 'buscar', 'muéstrame', 'muestrame', 'abre', 'archiv', 'finde', 'suche', 'zeig', 'zeige', 'öffne', 'archivio', 'nell’archivio', "nell'archivio", 'trova', 'cerca', 'mostrami', 'apri',
]);

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
  if (hasStem(LATEST_WORDS) && (asksWork || asksTale) && (verb || has(['покажи', 'show', 'muestra', 'muéstrame', 'enséñame', 'zeig', 'mostra']))) {
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
    paragraphs: [say(lang, `Открываю: ${quote(label)}.`, `Відкриваю: ${quote(label)}.`, `Opening ${quote(label)}.`, `Abro ${quote(label)}.`, `Ich öffne ${quote(label)}.`, `Apro ${quote(label)}.`)],
    links: [{ kind: 'section', label, hint, href }],
    action: { kind: 'go', href },
  });
  const arrow = say(lang, 'перейти', 'перейти', 'go', 'ir', 'öffnen', 'vai');

  switch (spec.kind) {
    case 'page': {
      const label = say(lang, ...spec.label);
      return open(label, spec.href, arrow);
    }
    case 'archive': {
      const href = `/figurines?q=${encodeURIComponent(spec.topic)}`;
      return {
        intent: 'go',
        paragraphs: [say(lang, `Ищу ${quote(spec.topic)} в архиве работ.`, `Шукаю ${quote(spec.topic)} в архіві робіт.`, `Searching the archive for ${quote(spec.topic)}.`, `Busco ${quote(spec.topic)} en el archivo de obras.`, `Ich suche ${quote(spec.topic)} im Archiv der Werke.`, `Cerco ${quote(spec.topic)} nell’archivio delle opere.`)],
        links: [{ kind: 'section', label: say(lang, 'Архив работ', 'Архів робіт', 'The archive', 'Archivo de obras', 'Archiv der Werke', 'Archivio delle opere'), hint: spec.topic, href }],
        action: { kind: 'go', href },
      };
    }
    case 'latest': {
      if (spec.what === 'tale') {
        const tale = talesNewest(index)[0];
        if (tale) return open(leafTitle(tale, lang), leafHref(tale, 'site_ask'), say(lang, 'новая небылица', 'нова небилиця', 'newest tall tale', 'cuento más reciente', 'neueste Geschichte', 'racconto più recente'));
        return open(say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), '/tales', arrow);
      }
      const work = newest(index.works, (w) => w.createdAt)[0];
      if (work) return open(workName(work), figurineHref(work, 'site_ask'), say(lang, 'новая работа', 'нова робота', 'newest work', 'obra más reciente', 'neuestes Werk', 'opera più recente'));
      return open(say(lang, 'Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'), '/figurines', arrow);
    }
    case 'random': {
      if (spec.what === 'tale') {
        const tales = index.tales.filter((l) => leafCopy(l, lang).body);
        const pick = tales[Math.floor(Math.random() * tales.length)];
        if (pick) return open(leafTitle(pick, lang), leafHref(pick, 'site_ask'), say(lang, 'случайная небылица', 'випадкова небилиця', 'random tall tale', 'cuento al azar', 'zufällige Geschichte', 'racconto a caso'));
        return open(say(lang, 'Небылицы', 'Небилиці', 'Tall tales', 'Cuentos', 'Geschichten', 'Racconti'), '/tales', arrow);
      }
      const works = index.works;
      const pick = works[Math.floor(Math.random() * works.length)];
      if (pick) return open(workName(pick), figurineHref(pick, 'site_ask'), say(lang, 'случайная работа', 'випадкова робота', 'random work', 'obra al azar', 'zufälliges Werk', 'opera a caso'));
      return open(say(lang, 'Все работы', 'Усі роботи', 'All works', 'Todas las obras', 'Alle Werke', 'Tutte le opere'), '/figurines', arrow);
    }
    case 'lang': {
      // Подтверждение — на том языке, на который переключились: его и увидят дальше.
      const text: Record<Lang, string> = {
        en: 'Switched to English.',
        ru: 'Переключил на русский.',
        uk: 'Перемкнув на українську.',
        es: 'Cambiado a español.',
        de: 'Auf Deutsch umgestellt.',
        it: 'Passato all’italiano.',
      };
      return {
        intent: 'go',
        paragraphs: [text[spec.lang]],
        links: [],
        action: { kind: 'lang', lang: spec.lang } satisfies AskAction,
      };
    }
  }
}
