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
import { loadIndex } from '$lib/site-index';
import { answer, needsMeaning, parseAsk, type AskReply } from '$lib/site-ask';
import type { AskAction } from '$lib/site-ask-kit';

export interface AskTurn {
  id: number;
  question: string;
  /** `null`, пока ответ собирается или если не вышло (`failed`). */
  reply: AskReply | null;
  failed: boolean;
  /** Ответ напечатан до конца. Открытое заново окно не печатает его второй раз. */
  typed: boolean;
}

const TURNS_MAX = 8;
export const QUESTION_MAX = 200;
const THINK_MS = 650;
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

  async ask(raw: string, via: 'input' | 'chip') {
    const question = raw.trim().slice(0, QUESTION_MAX);
    if (!question) return;

    // Новый вопрос: прежний ответ достаточно читали, дописываем его сразу,
    // а команду, которую он обещал, больше не выполняем.
    this.cancelAction();
    for (const t of this.turns) t.typed = true;

    const id = this.#next++;
    this.turns = [...this.turns, { id, question, reply: null, failed: false, typed: false }].slice(-TURNS_MAX);

    const parsed = parseAsk(question);
    // В замер уходит род вопроса, а не его текст.
    interact('site_ask', `${via}:${parsed.intent}`);

    const started = performance.now();
    const [index, meaning, gazetteTotal] = await Promise.all([
      loadIndex().catch(() => null),
      needsMeaning(parsed)
        ? withTimeout(
            api.semanticSearch(parsed.topic, 24).then((hits) => hits.map((h) => h.id)),
            MEANING_WAIT_MS,
          ).catch(() => null)
        : null,
      parsed.intent === 'summary' || parsed.intent === 'count'
        ? api.getSectionCounts().then((c) => c.gazette.total).catch(() => null)
        : null,
    ]);

    if (!prefersReducedMotion()) {
      const left = THINK_MS - (performance.now() - started);
      if (left > 0) await sleep(left);
    }

    const turn = this.turns.find((t) => t.id === id);
    if (!turn) return;
    if (!index) {
      turn.failed = true;
      turn.typed = true;
      return;
    }
    try {
      const say = get(t);
      turn.reply = answer(index, question, get(lang), {
        meaning,
        gazetteTotal,
        text: (key) => say(key as Parameters<typeof say>[0]),
        page: { path: page.url.pathname, card: page.url.searchParams.get('card') },
      });
      if (turn.reply.action) this.#schedule(turn.reply.action);
      if (turn.reply.missed) {
        // Только вопросы, на которые ответа не нашлось: по ним видно, чему учить сайт.
        const text = scrubQuestion(question);
        if (text) interact('site_ask_miss', text, 0);
      }
    } catch {
      turn.failed = true;
      turn.typed = true;
    }
  }
}

export const siteAsk = new SiteAskStore();
