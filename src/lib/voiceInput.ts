// Голосовой ввод вопроса: распознавание речи самого браузера.
//
// Сайт звук не принимает и не хранит: распознаёт его браузер. В Chrome это
// делает сервис Google, в Safari — Apple; на наши серверы звук не попадает,
// туда приходит только готовый текст вопроса, как будто его набрали. Об этом
// сказано в подсказке у кнопки и в политике конфиденциальности.
//
// Микрофон просится только по нажатию на кнопку и выключается, как только
// фраза закончилась или окно закрыли.
import type { Lang } from '$lib/i18n';

/** То немногое из `SpeechRecognition`, что здесь нужно: в типах TypeScript его нет. */
interface Recognizer {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  maxAlternatives: number;
  onresult: ((e: RecognizerEvent) => void) | null;
  onerror: ((e: { error: string }) => void) | null;
  onend: (() => void) | null;
  start(): void;
  stop(): void;
  abort(): void;
}

interface RecognizerEvent {
  resultIndex: number;
  results: ArrayLike<ArrayLike<{ transcript: string }> & { isFinal: boolean }>;
}

type RecognizerCtor = new () => Recognizer;

const TAGS: Record<Lang, string> = { ru: 'ru-RU', uk: 'uk-UA', en: 'en-US', es: 'es-ES', de: 'de-DE', it: 'it-IT' };

function ctor(): RecognizerCtor | null {
  if (typeof window === 'undefined') return null;
  const w = window as unknown as { SpeechRecognition?: RecognizerCtor; webkitSpeechRecognition?: RecognizerCtor };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition ?? null;
}

export const voiceSupported = (): boolean => ctor() !== null;

export type VoiceError = 'denied' | 'failed' | 'silence';

export interface VoiceHandlers {
  /** Текст растёт по ходу речи; `final` — фраза закончена. */
  onText: (text: string, final: boolean) => void;
  /** Микрофон выключен: и после фразы, и после ошибки, и после `stop`. */
  onEnd: () => void;
  onError: (error: VoiceError) => void;
}

export interface Voice {
  stop: () => void;
}

export function startVoice(lang: Lang, h: VoiceHandlers): Voice | null {
  const Ctor = ctor();
  if (!Ctor) return null;
  const rec = new Ctor();
  rec.lang = TAGS[lang];
  rec.continuous = false;
  rec.interimResults = true;
  rec.maxAlternatives = 1;

  let ended = false;
  const end = () => {
    if (ended) return;
    ended = true;
    h.onEnd();
  };

  rec.onresult = (e) => {
    let text = '';
    let final = false;
    for (let i = 0; i < e.results.length; i += 1) {
      text += e.results[i][0].transcript;
      if (e.results[i].isFinal) final = true;
    }
    h.onText(text.trim(), final);
  };
  rec.onerror = (e) => {
    if (e.error === 'not-allowed' || e.error === 'service-not-allowed') h.onError('denied');
    else if (e.error === 'no-speech' || e.error === 'aborted') h.onError('silence');
    else h.onError('failed');
  };
  rec.onend = end;

  try {
    rec.start();
  } catch {
    h.onError('failed');
    end();
    return null;
  }
  return {
    stop: () => {
      try {
        rec.abort();
      } catch {
        // уже остановлен
      }
      end();
    },
  };
}
