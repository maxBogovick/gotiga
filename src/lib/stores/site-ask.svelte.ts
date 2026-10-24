// Разговор в окне «Спросить сайт»: вопросы, ответы и то, дописан ли ответ.
//
// Ответ собирается сразу, а выдерживается пауза (`THINK_MS`) и печатается по
// словам — так он читается как ответ, а не как таблица, выскочившая целиком.
// Под `prefers-reduced-motion` ни паузы, ни печатания нет.
import { get } from 'svelte/store';
import { page } from '$app/state';
import { goto } from '$app/navigation';
import { api } from '$lib/api';
import { lang, t, loadLang } from '$lib/i18n';
import { interact } from '$lib/analytics';
import { loadIndex, indexIn } from '$lib/site-index';
import {
  answer, dataReply, isLoose, needsMeaning, ownedByRules, parseAsk, suggestionLinks, textReply, unansweredReply,
  type AskReply,
} from '$lib/site-ask';
import type { AskRouteReply } from '$lib/types/api';
import type { AskAction } from '$lib/site-ask-kit';

export interface AskTurn {
  id: number;
  question: string;
  /** `null`, пока ответ собирается или если не вышло (`failed`). */
  reply: AskReply | null;
  failed: boolean;
  /** Ответ напечатан до конца. Открытое заново окно не печатает его второй раз. */
  typed: boolean;
  /** Ответ показан, пока человек ещё печатает: следующий заменяет его, а в замер
   *  и в исполнение команд он попадает, только когда вопрос отправлен (`commitLive`). */
  live: boolean;
  /** Род вопроса и решение каталога: уходят в замер при отправке. */
  kind: string;
  note: string | null;
}

const TURNS_MAX = 8;
export const QUESTION_MAX = 200;
const THINK_MS = 650;
/** Ответ, который показывают на лету, ждёт меньше: человек его не просил. */
const THINK_LIVE_MS = 250;
/** Смысловой поиск сервера считает дольше словаря: ждём его не дольше этого. */
const MEANING_WAIT_MS = 2500;

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T | null> {
  return Promise.race([p, new Promise<null>((resolve) => setTimeout(() => resolve(null), ms))]);
}

/** Вопрос без ответа уходит в замер: без адресов, номеров и длиннее ста знаков не бывает. */
function scrubQuestion(raw: string): string {
  return raw
    .split(/\s+/)
    .filter((w) => !w.includes('@') && !/\d{5,}/.test(w.replace(/[\s().+-]/g, '')) && !/^https?:/i.test(w))
    .join(' ')
    .trim()
    .slice(0, 100);
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Сколько человек успевает прочесть «Открываю …» и передумать. */
const ACTION_MS = 1100;

class SiteAskStore {
  turns = $state<AskTurn[]>([]);
  #next = 1;
  #action: ReturnType<typeof setTimeout> | null = null;

  /** Отменить команду, которую окно собиралось выполнить: передумали, закрыли окно. */
  cancelAction() {
    if (this.#action) clearTimeout(this.#action);
    this.#action = null;
  }

  #schedule(action: AskAction) {
    this.cancelAction();
    this.#action = setTimeout(() => {
      this.#action = null;
      if (action.kind === 'go') {
        void goto(action.href);
      } else {
        void loadLang(action.lang).then(() => lang.set(action.lang));
      }
    }, prefersReducedMotion() ? 400 : ACTION_MS);
  }

  /** Ответ дописан (или человека он больше не интересует): показать целиком. */
  markTyped(id: number) {
    const turn = this.turns.find((t) => t.id === id);
    if (turn) turn.typed = true;
  }

  /** Ответ по записи каталога: готовый текст или данные сайта. `null` — записи
   *  нечего сказать (пустой текст), тогда отвечают правила по словам. */
  async #fromCatalog(
    idx: Parameters<typeof dataReply>[0],
    l: Parameters<typeof dataReply>[1],
    question: string,
    routed: AskRouteReply,
    gazetteTotal: number | null,
  ): Promise<AskReply | null> {
    const entry = routed.answer;
    if (!entry) return null;
    if (entry.handler) {
      const total = entry.handler === 'count' && gazetteTotal === null
        ? await api.getSectionCounts().then((c) => c.gazette.total).catch(() => null)
        : gazetteTotal;
      return dataReply(idx, l, entry.handler, question, total);
    }
    return textReply(entry);
  }

  /** Вопрос отправлен: ответ, показанный на лету, становится обычным. Команда,
   *  которую он обещал, выполняется теперь, а замер получает то же, что получил
   *  бы после прямого вопроса. Повторный вызов ничего не делает. */
  commitLive() {
    const turn = this.turns.find((x) => x.live);
    if (!turn) return;
    turn.live = false;
    interact('site_ask', `input:${turn.kind}`);
    if (turn.note) interact('site_ask_route', turn.note);
    // Ответ ещё собирается: остальное доделает `ask`, когда он будет готов.
    if (!turn.reply) return;
    this.#finish(turn);
  }

  /** Очистить разговор: вопросы, ответы и команду, которую ответ обещал. Вопрос,
   *  на который ответ ещё собирается, дособирается вхолостую: его строки уже нет. */
  clear() {
    this.cancelAction();
    this.turns = [];
  }

  /** Убрать ответ «на лету», если человек стёр вопрос или ушёл. */
  dropLive() {
    if (!this.turns.some((x) => x.live)) return;
    this.cancelAction();
    this.turns = this.turns.filter((x) => !x.live);
  }

  /** После готового ответа: команда и запись вопроса без ответа. */
  #finish(turn: AskTurn) {
    if (turn.reply?.action) this.#schedule(turn.reply.action);
    if (turn.reply?.missed) {
      // Только вопросы, на которые ответа не нашлось: по ним видно, чему учить сайт.
      const text = scrubQuestion(turn.question);
      if (text) interact('site_ask_miss', text, 0);
    }
  }

  async ask(raw: string, via: 'input' | 'chip' | 'voice' | 'live', quiz?: string[]) {
    const question = raw.trim().slice(0, QUESTION_MAX);
    if (!question) return;
    const live = via === 'live';

    // Новый вопрос: прежний ответ достаточно читали, дописываем его сразу,
    // а команду, которую он обещал, больше не выполняем. Ответ на лету, который
    // не отправили, заменяется: иначе в разговоре осталась бы каждая
    // промежуточная фраза («как», «как зак», «как заказать»).
    this.cancelAction();
    const kept = this.turns.filter((x) => !x.live);
    for (const t of kept) t.typed = true;

    const id = this.#next++;
    const parsed = parseAsk(question);
    const kind = quiz ? 'match' : parsed.intent;
    this.turns = [
      ...kept,
      { id, question, reply: null, failed: false, typed: false, live, kind, note: null },
    ].slice(-TURNS_MAX);
    // В замер уходит род вопроса, а не его текст. Выбор в подборе — тоже подбор.
    if (!live) interact('site_ask', `${via}:${kind}`);

    const started = performance.now();
    // Правила, у которых точные приметы (страница, команда, подбор, приветствие),
    // идут первыми и каталога не спрашивают.
    const asksCatalog = !quiz && !ownedByRules(parsed.intent);
    const [index, meaning, gazetteTotal, routed] = await Promise.all([
      loadIndex().catch(() => null),
      // Ответ на лету смысловым поиском не пользуется: он нужен только поиску по
      // теме, а её ответ на лету не показывается.
      !quiz && !live && needsMeaning(parsed)
        ? withTimeout(
            api.semanticSearch(parsed.topic, 24).then((hits) => hits.map((h) => h.id)),
            MEANING_WAIT_MS,
          ).catch(() => null)
        : null,
      parsed.intent === 'summary' || parsed.intent === 'count'
        ? api.getSectionCounts().then((c) => c.gazette.total).catch(() => null)
        : null,
      asksCatalog ? api.askRoute(question, get(lang)) : null,
    ]);
    const note = asksCatalog ? (routed ? `${routed.decision}:${routed.answer?.key ?? '-'}` : 'off') : null;
    if (note && !live) interact('site_ask_route', note);

    if (!prefersReducedMotion()) {
      const left = (live ? THINK_LIVE_MS : THINK_MS) - (performance.now() - started);
      if (left > 0) await sleep(left);
    }

    const turn = this.turns.find((t) => t.id === id);
    if (!turn) return;
    turn.note = note;
    if (!index) {
      turn.failed = true;
      turn.typed = true;
      return;
    }
    try {
      const say = get(t);
      const l = get(lang);
      const idx = indexIn(index, l);
      const fromCatalog =
        routed?.decision === 'unanswered'
          ? unansweredReply(l)
          : routed?.decision === 'answer' && routed.answer
            ? await this.#fromCatalog(idx, l, question, routed, gazetteTotal)
            : null;
      let reply: AskReply =
        fromCatalog ??
        answer(idx, question, l, {
          meaning,
          gazetteTotal,
          text: (key) => say(key as Parameters<typeof say>[0]),
          page: { path: page.url.pathname, card: page.url.searchParams.get('card') },
          quiz,
        });
      // Каталог не уверен: вместо «не умею» — три ближайших вопроса, а под
      // ответом по одному слову («сколько делается работа» → «сколько работ») —
      // «возможно, вы спрашивали».
      const offered = suggestionLinks(routed, question);
      if (!fromCatalog && offered.length) {
        if (reply.missed) reply = { ...reply, links: offered };
        else if (isLoose(reply.intent)) reply = { ...reply, suggest: offered };
      }
      turn.reply = reply;
      // Ответ на лету ждёт отправки вопроса: команду и запись «без ответа»
      // делает `commitLive`.
      if (!turn.live) this.#finish(turn);
    } catch {
      turn.failed = true;
      turn.typed = true;
    }
  }
}

export const siteAsk = new SiteAskStore();
